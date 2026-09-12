#pragma once

#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

// Host simulator compatibility: allocation capabilities have no equivalent on
// the desktop, so retain the API while using the process heap.
#define MALLOC_CAP_8BIT 0x00000004u
#define MALLOC_CAP_SPIRAM 0x00000400u
#define MALLOC_CAP_INTERNAL 0x00000800u

static inline void *heap_caps_malloc(size_t size, uint32_t) {
  return malloc(size);
}

static inline void *heap_caps_calloc(size_t count, size_t size, uint32_t) {
  return calloc(count, size);
}

static inline void *heap_caps_realloc(void *ptr, size_t size, uint32_t) {
  return realloc(ptr, size);
}

static inline void heap_caps_free(void *ptr) { free(ptr); }
