#include "ota_proxy.h"

#include <cstring>
#include "esp_partition.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "can_link.h"
#include "common/crc.hpp"
#include "common/messages.hpp"
#include "core/core.h"

namespace ota_proxy {
namespace {
constexpr size_t kHeaderBytes = 4096, kBlockBytes = 2048;
constexpr int64_t kAckTimeoutUs = 5 * 1000 * 1000;
struct Header { uint32_t magic, size, crc, valid; };
constexpr uint32_t kMagic = 0x4f544153, kValid = 0xa55aa55a;
struct State { const esp_partition_t* partition=nullptr; uint32_t size=0,written=0,sent=0; common::Crc32Incremental crc; bool uploading=false,sending=false,end_ack=false; uint16_t ack_block=0xffff,ack_crc=0; };
State g;
void send_ctrl(common::FlashSubCmd cmd, uint32_t value=0) { common::FlashCtrlPayload p{}; p.subcmd=cmd; if(cmd==common::FlashSubCmd::kBegin) p.image_size=value; if(cmd==common::FlashSubCmd::kEnd) p.image_crc32=value; common::Frame f=p.pack(); can_link::send_message(common::MessageType::kFlashCtrl,common::Dest::kSensors,f.data(),8); }
bool wait_block(uint16_t n,uint16_t crc) { int64_t d=esp_timer_get_time()+kAckTimeoutUs; while(esp_timer_get_time()<d) { if(g.ack_block==n && (n==0 || g.ack_crc==crc)) return true; vTaskDelay(pdMS_TO_TICKS(20)); } return false; }
void sender_task(void*) {
  uint8_t block[kBlockBytes]; g.ack_block=0xffff; send_ctrl(common::FlashSubCmd::kBegin,g.size); if(!wait_block(0,0)) goto fail;
  for(uint16_t n=1;g.sent<g.size;++n) { size_t count=(g.size-g.sent<kBlockBytes)?g.size-g.sent:kBlockBytes; if(esp_partition_read(g.partition,kHeaderBytes+g.sent,block,count)!=ESP_OK) goto fail; uint16_t crc=common::crc16_ccitt(block,count); bool ok=false; for(int a=0;a<3&&!ok;++a) { g.ack_block=0xffff; for(size_t o=0;o<count;o+=8) { uint8_t bytes=static_cast<uint8_t>((count-o<8)?count-o:8); can_link::send_message(common::MessageType::kFlashData,common::Dest::kSensors,block+o,bytes); vTaskDelay(pdMS_TO_TICKS(2)); } ok=wait_block(n,crc); } if(!ok) goto fail; g.sent+=count; core::update_flash_progress(g.sent); }
  g.end_ack=false; send_ctrl(common::FlashSubCmd::kEnd,g.crc.finish()); { int64_t d=esp_timer_get_time()+kAckTimeoutUs; while(!g.end_ack && esp_timer_get_time()<d) vTaskDelay(pdMS_TO_TICKS(20)); if(!g.end_ack) goto fail; }
  g.sending=false; core::finish_flash(); vTaskDelete(nullptr); return;
fail: send_ctrl(common::FlashSubCmd::kAbort); g.sending=false; core::finish_flash(); vTaskDelete(nullptr);
}
}  // namespace
bool begin_upload(uint32_t size) { if(g.uploading||g.sending||size==0) return false; g.partition=esp_partition_find_first(ESP_PARTITION_TYPE_DATA,static_cast<esp_partition_subtype_t>(0x40),"ota_staging"); /* Les cartes déjà livrées ont encore assets: le repli permet de déployer cette image OTA avant la prochaine écriture USB de la table. */ if(!g.partition) g.partition=esp_partition_find_first(ESP_PARTITION_TYPE_DATA,ESP_PARTITION_SUBTYPE_DATA_SPIFFS,"assets"); if(!g.partition||size>g.partition->size-kHeaderBytes||esp_partition_erase_range(g.partition,0,g.partition->size)!=ESP_OK) return false; g.size=size;g.written=g.sent=0;g.crc=common::Crc32Incremental{};g.uploading=true;return true; }
bool write_upload(const uint8_t* data,size_t len) { if(!g.uploading||len>g.size-g.written||esp_partition_write(g.partition,kHeaderBytes+g.written,data,len)!=ESP_OK) return false;g.crc.update(data,len);g.written+=len;core::update_flash_progress(g.written);return true; }
bool commit_upload() { if(!g.uploading||g.written!=g.size) return false; Header h{kMagic,g.size,g.crc.finish(),kValid}; if(esp_partition_write(g.partition,0,&h,sizeof(h))!=ESP_OK) return false;g.uploading=false;g.sending=true;return xTaskCreatePinnedToCore(sender_task,"ota_proxy",6144,nullptr,5,nullptr,0)==pdPASS; }
void abort_upload(){g.uploading=false;if(!g.sending)core::finish_flash();}
bool active(){return g.uploading||g.sending;}
void on_flash_ctrl_received(const uint8_t* data,size_t len){common::FlashCtrlPayload p;if(!common::FlashCtrlPayload::unpack(data,len,&p)||!g.sending)return;if(p.subcmd==common::FlashSubCmd::kBlockAck){g.ack_block=p.block_number;g.ack_crc=p.block_crc16;}if(p.subcmd==common::FlashSubCmd::kEnd&&g.sent==g.size)g.end_ack=true;}
}  // namespace ota_proxy
