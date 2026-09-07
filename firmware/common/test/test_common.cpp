// Tests hôte de firmware/common/ — aucune dépendance ESP-IDF, compile et
// tourne sur le Mac (voir docs/firmware-implementation.md, phase 0).

#include <cstdio>
#include <cstdlib>
#include <string>

#include "common/crc.hpp"
#include "common/framing.hpp"
#include "common/messages.hpp"
#include "common/protocol.hpp"
#include "log_codes.hpp"

namespace {

int g_failures = 0;

#define CHECK(cond)                                                    \
  do {                                                                 \
    if (!(cond)) {                                                     \
      std::fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__,     \
                   #cond);                                             \
      ++g_failures;                                                    \
    }                                                                  \
  } while (0)

using namespace common;

void test_can_id_roundtrip() {
  const CanId id{MessageType::kSet, Dest::kSensors, Node::kScreen};
  const uint16_t raw = encode_can_id(id);
  CHECK(raw <= 0x7FF);  // 11 bits
  const CanId back = decode_can_id(raw);
  CHECK(back.type == id.type);
  CHECK(back.dest == id.dest);
  CHECK(back.src == id.src);
}

void test_can_id_priority_ordering() {
  // STOP (0x00) doit gagner l'arbitrage face à FLASH_DATA (0x39) : un ID
  // numériquement plus petit gagne l'arbitrage CAN.
  const uint16_t stop = encode_can_id({MessageType::kStop, Dest::kSensors, Node::kScreen});
  const uint16_t flash = encode_can_id({MessageType::kFlashData, Dest::kSensors, Node::kScreen});
  CHECK(stop < flash);
}

void test_known_message_type() {
  CHECK(is_known_message_type(static_cast<uint8_t>(MessageType::kPing)));
  CHECK(!is_known_message_type(0x3F));  // valeur haute, jamais assignée
}

void test_set_payload_roundtrip() {
  SetPayload in{};
  in.set_ssr = true;
  in.set_dimmer = true;
  in.ssr = true;
  in.dimmer = 42;
  in.ttl_ms = 500;
  const Frame f = in.pack();

  SetPayload out{};
  CHECK(SetPayload::unpack(f.data(), f.size(), &out));
  CHECK(out.set_ssr == in.set_ssr);
  CHECK(out.set_dimmer == in.set_dimmer);
  CHECK(out.ssr == in.ssr);
  CHECK(out.dimmer == in.dimmer);
  CHECK(out.ttl_ms == in.ttl_ms);
}

void test_pong_payload_roundtrip() {
  PongPayload in{};
  in.node = Node::kSensors;
  in.version_major = 1;
  in.version_minor = 2;
  in.version_patch = 3;
  in.uptime_s = 123456789;
  const Frame f = in.pack();

  PongPayload out{};
  CHECK(PongPayload::unpack(f.data(), f.size(), &out));
  CHECK(out.node == in.node);
  CHECK(out.version_major == in.version_major);
  CHECK(out.version_minor == in.version_minor);
  CHECK(out.version_patch == in.version_patch);
  CHECK(out.uptime_s == in.uptime_s);
}

void test_reqstatus_payload_roundtrip() {
  ReqStatusPayload in{};
  in.target_type = MessageType::kStatusFlow;
  in.period_ms = 200;
  const Frame f = in.pack();

  ReqStatusPayload out{};
  CHECK(ReqStatusPayload::unpack(f.data(), f.size(), &out));
  CHECK(out.target_type == in.target_type);
  CHECK(out.period_ms == in.period_ms);
}

void test_status_pressure_payload_roundtrip() {
  StatusPressurePayload in{};
  in.pressure_raw = 0x00ABCDEF & 0xFFFFFF;
  in.temperature_raw = 0x1234;
  in.timestamp_ms = 60000;
  in.flags = 0b01;
  const Frame f = in.pack();

  StatusPressurePayload out{};
  CHECK(StatusPressurePayload::unpack(f.data(), f.size(), &out));
  CHECK(out.pressure_raw == in.pressure_raw);
  CHECK(out.temperature_raw == in.temperature_raw);
  CHECK(out.timestamp_ms == in.timestamp_ms);
  CHECK(out.flags == in.flags);
}

void test_status_flow_payload_roundtrip() {
  StatusFlowPayload in{};
  in.pulse_count = 4000000000u;
  in.last_edge_ms = 1234;
  in.flags = 1;
  const Frame f = in.pack();

  StatusFlowPayload out{};
  CHECK(StatusFlowPayload::unpack(f.data(), f.size(), &out));
  CHECK(out.pulse_count == in.pulse_count);
  CHECK(out.last_edge_ms == in.last_edge_ms);
  CHECK(out.flags == in.flags);
}

void test_status_actuators_payload_roundtrip() {
  StatusActuatorsPayload in{};
  in.ssr = true;
  in.dimmer = 77;
  in.lease_remaining_ms = 480;
  in.continuous_on_ms = 59000;
  in.flags = 0b101;
  const Frame f = in.pack();

  StatusActuatorsPayload out{};
  CHECK(StatusActuatorsPayload::unpack(f.data(), f.size(), &out));
  CHECK(out.ssr == in.ssr);
  CHECK(out.dimmer == in.dimmer);
  CHECK(out.lease_remaining_ms == in.lease_remaining_ms);
  CHECK(out.continuous_on_ms == in.continuous_on_ms);
  CHECK(out.flags == in.flags);
}

void test_log_payload_roundtrip() {
  LogPayload in{};
  in.code = static_cast<uint8_t>(LogCode::kRuntimeLockoutTriggered);
  in.severity = static_cast<uint8_t>(LogSeverity::kError);
  in.arg16 = 0;
  in.arg32 = 60000;
  const Frame f = in.pack();

  LogPayload out{};
  CHECK(LogPayload::unpack(f.data(), f.size(), &out));
  CHECK(out.code == in.code);
  CHECK(out.severity == in.severity);
  CHECK(out.arg32 == in.arg32);
  CHECK(std::string(log_code_name(static_cast<LogCode>(out.code))) ==
        "RUNTIME_LOCKOUT_TRIGGERED");
}

void test_flash_ctrl_payload_roundtrip() {
  {
    FlashCtrlPayload in{};
    in.subcmd = FlashSubCmd::kBegin;
    in.image_size = 1048576;
    const Frame f = in.pack();
    FlashCtrlPayload out{};
    CHECK(FlashCtrlPayload::unpack(f.data(), f.size(), &out));
    CHECK(out.subcmd == FlashSubCmd::kBegin);
    CHECK(out.image_size == in.image_size);
  }
  {
    FlashCtrlPayload in{};
    in.subcmd = FlashSubCmd::kBlockAck;
    in.block_number = 42;
    in.block_crc16 = 0xBEEF;
    const Frame f = in.pack();
    FlashCtrlPayload out{};
    CHECK(FlashCtrlPayload::unpack(f.data(), f.size(), &out));
    CHECK(out.block_number == in.block_number);
    CHECK(out.block_crc16 == in.block_crc16);
  }
  {
    FlashCtrlPayload in{};
    in.subcmd = FlashSubCmd::kEnd;
    in.image_crc32 = 0xDEADBEEF;
    const Frame f = in.pack();
    FlashCtrlPayload out{};
    CHECK(FlashCtrlPayload::unpack(f.data(), f.size(), &out));
    CHECK(out.image_crc32 == in.image_crc32);
  }
}

void test_framing_pdu_roundtrip() {
  RawFrame in{};
  in.can_id = encode_can_id({MessageType::kPong, Dest::kScreen, Node::kSensors});
  in.dlc = 8;
  for (int i = 0; i < 8; ++i) in.data[static_cast<size_t>(i)] = static_cast<uint8_t>(0x10 + i);

  uint8_t pdu[kMaxPduSize];
  const size_t pdu_len = encode_pdu(in, pdu);
  CHECK(pdu_len == 3 + in.dlc + 2);

  RawFrame out{};
  CHECK(decode_pdu(pdu, pdu_len, &out));
  CHECK(out.can_id == in.can_id);
  CHECK(out.dlc == in.dlc);
  CHECK(out.data == in.data);
}

void test_framing_pdu_rejects_bad_crc() {
  RawFrame in{};
  in.can_id = encode_can_id({MessageType::kPing, Dest::kBroadcast, Node::kScreen});
  in.dlc = 0;
  uint8_t pdu[kMaxPduSize];
  const size_t pdu_len = encode_pdu(in, pdu);
  pdu[pdu_len - 1] ^= 0xFF;  // corrompt le CRC
  RawFrame out{};
  CHECK(!decode_pdu(pdu, pdu_len, &out));
}

void test_cobs_roundtrip_with_zeros() {
  // Un bloc FLASH_DATA plein de zéros est le cas que COBS doit absorber.
  uint8_t data[kMaxPduSize] = {0};
  data[1] = 0x05;  // un seul octet non nul au milieu

  uint8_t encoded[kMaxCobsSize];
  const size_t encoded_len = cobs_encode(data, sizeof(data), encoded);
  for (size_t i = 0; i < encoded_len; ++i) CHECK(encoded[i] != 0x00);

  uint8_t decoded[kMaxPduSize];
  const size_t decoded_len = cobs_decode(encoded, encoded_len, decoded);
  CHECK(decoded_len == sizeof(data));
  for (size_t i = 0; i < decoded_len; ++i) CHECK(decoded[i] == data[i]);
}

void test_encode_framed_roundtrip_via_stream_decoder() {
  RawFrame in{};
  in.can_id = encode_can_id({MessageType::kSet, Dest::kSensors, Node::kScreen});
  in.dlc = 5;
  in.data = {0x03, 0x01, 42, 0xF4, 0x01, 0, 0, 0};

  uint8_t framed[kMaxCobsSize + 1];
  const size_t framed_len = encode_framed(in, framed);
  CHECK(framed_len > 0);
  CHECK(framed[framed_len - 1] == 0x00);

  static RawFrame received{};
  static int received_count = 0;
  received_count = 0;
  StreamDecoder decoder;
  for (size_t i = 0; i < framed_len; ++i) {
    decoder.push_byte(framed[i], [](const RawFrame& f, void*) {
      received = f;
      ++received_count;
    }, nullptr);
  }
  CHECK(received_count == 1);
  CHECK(received.can_id == in.can_id);
  CHECK(received.dlc == in.dlc);
  CHECK(received.data == in.data);
  CHECK(decoder.dropped_count == 0);
}

void test_stream_decoder_resyncs_after_garbage() {
  // Un octet perdu (ou un branchement en cours de trame) ne doit coûter que
  // la trame en cours, pas bloquer les suivantes.
  RawFrame in{};
  in.can_id = encode_can_id({MessageType::kPing, Dest::kBroadcast, Node::kSensors});
  in.dlc = 0;

  uint8_t framed[kMaxCobsSize + 1];
  const size_t framed_len = encode_framed(in, framed);

  static int received_count = 0;
  received_count = 0;
  StreamDecoder decoder;

  const uint8_t garbage[] = {0x7A, 0x11, 0x00};  // trame invalide, close par 0x00
  for (uint8_t b : garbage) {
    decoder.push_byte(b, [](const RawFrame&, void*) { ++received_count; }, nullptr);
  }
  CHECK(received_count == 0);
  CHECK(decoder.dropped_count == 1);

  for (size_t i = 0; i < framed_len; ++i) {
    decoder.push_byte(framed[i], [](const RawFrame&, void*) { ++received_count; }, nullptr);
  }
  CHECK(received_count == 1);
}

void test_crc16_known_vector() {
  // "123456789" -> 0x29B1 en CRC-16/CCITT-FALSE (poly 0x1021, init 0xFFFF).
  const uint8_t data[] = "123456789";
  CHECK(crc16_ccitt(data, 9) == 0x29B1);
}

void test_crc32_known_vector() {
  // "123456789" -> 0xCBF43926 en CRC-32 (poly 0xEDB88320, IEEE 802.3).
  const uint8_t data[] = "123456789";
  CHECK(crc32_ieee(data, 9) == 0xCBF43926u);
}

}  // namespace

int main() {
  test_can_id_roundtrip();
  test_can_id_priority_ordering();
  test_known_message_type();
  test_set_payload_roundtrip();
  test_pong_payload_roundtrip();
  test_reqstatus_payload_roundtrip();
  test_status_pressure_payload_roundtrip();
  test_status_flow_payload_roundtrip();
  test_status_actuators_payload_roundtrip();
  test_log_payload_roundtrip();
  test_flash_ctrl_payload_roundtrip();
  test_framing_pdu_roundtrip();
  test_framing_pdu_rejects_bad_crc();
  test_cobs_roundtrip_with_zeros();
  test_encode_framed_roundtrip_via_stream_decoder();
  test_stream_decoder_resyncs_after_garbage();
  test_crc16_known_vector();
  test_crc32_known_vector();

  if (g_failures == 0) {
    std::printf("OK\n");
    return 0;
  }
  std::fprintf(stderr, "%d échec(s)\n", g_failures);
  return 1;
}
