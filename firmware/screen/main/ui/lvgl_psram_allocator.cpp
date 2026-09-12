#include <cstddef>

#include "esp_heap_caps.h"
#include "lvgl.h"

// LVGL's objects, event descriptors and non-static label text are numerous
// but neither DMA nor ISR data. Keeping them in PSRAM preserves the scarce
// internal DMA-capable heap needed by the RGB panel's bounce buffers.
namespace {
constexpr uint32_t kLvglCaps = MALLOC_CAP_SPIRAM | MALLOC_CAP_8BIT;
}

extern "C" {

void lv_mem_init(void) {}
void lv_mem_deinit(void) {}

lv_mem_pool_t lv_mem_add_pool(void *, size_t) { return nullptr; }
void lv_mem_remove_pool(lv_mem_pool_t) {}

void *lv_malloc_core(size_t size) {
  return heap_caps_malloc(size, kLvglCaps);
}

void *lv_realloc_core(void *ptr, size_t size) {
  return heap_caps_realloc(ptr, size, kLvglCaps);
}

void lv_free_core(void *ptr) { heap_caps_free(ptr); }

void lv_mem_monitor_core(lv_mem_monitor_t *monitor) {
  // heap_caps does not expose per-LVGL allocation accounting.
  if (monitor != nullptr) *monitor = {};
}

lv_result_t lv_mem_test_core(void) { return LV_RESULT_OK; }

}  // extern "C"
