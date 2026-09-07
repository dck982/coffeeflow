#include "common/framing.hpp"

#include "common/crc.hpp"

namespace common {

size_t encode_pdu(const RawFrame& frame, uint8_t* out) {
  out[0] = static_cast<uint8_t>(frame.can_id & 0xFF);
  out[1] = static_cast<uint8_t>((frame.can_id >> 8) & 0xFF);
  out[2] = frame.dlc;
  for (uint8_t i = 0; i < frame.dlc; ++i) {
    out[3 + i] = frame.data[i];
  }
  const size_t body_len = 3 + frame.dlc;
  const uint16_t crc = crc16_ccitt(out, body_len);
  out[body_len] = static_cast<uint8_t>(crc & 0xFF);
  out[body_len + 1] = static_cast<uint8_t>((crc >> 8) & 0xFF);
  return body_len + 2;
}

bool decode_pdu(const uint8_t* in, size_t len, RawFrame* out) {
  if (len < 5) return false;  // id(2) + dlc(1) + crc16(2), dlc == 0 au plancher
  const uint8_t dlc = in[2];
  if (dlc > 8) return false;
  const size_t expected = 3 + dlc + 2;
  if (len != expected) return false;

  const uint16_t crc = crc16_ccitt(in, 3 + dlc);
  const uint16_t got = static_cast<uint16_t>(in[3 + dlc]) |
                        (static_cast<uint16_t>(in[3 + dlc + 1]) << 8);
  if (crc != got) return false;

  const uint16_t can_id = static_cast<uint16_t>(in[0]) |
                           (static_cast<uint16_t>(in[1]) << 8);
  if (can_id > 0x7FF) return false;

  out->can_id = can_id;
  out->dlc = dlc;
  out->data.fill(0);
  for (uint8_t i = 0; i < dlc; ++i) {
    out->data[i] = in[3 + i];
  }
  return true;
}

// Référence : Cheshire & Baker, "Consistent Overhead Byte Stuffing".
size_t cobs_encode(const uint8_t* in, size_t len, uint8_t* out) {
  size_t read_index = 0;
  size_t write_index = 1;
  size_t code_index = 0;
  uint8_t code = 1;

  while (read_index < len) {
    if (in[read_index] == 0) {
      out[code_index] = code;
      code = 1;
      code_index = write_index++;
      ++read_index;
    } else {
      out[write_index++] = in[read_index++];
      ++code;
      if (code == 0xFF) {
        out[code_index] = code;
        code = 1;
        code_index = write_index++;
      }
    }
  }
  out[code_index] = code;
  return write_index;
}

size_t cobs_decode(const uint8_t* in, size_t len, uint8_t* out) {
  size_t read_index = 0;
  size_t write_index = 0;

  while (read_index < len) {
    const uint8_t code = in[read_index];
    if (code == 0) return 0;  // un 0x00 ne peut apparaître qu'en fin de trame
    if (read_index + code > len && code != 1) return 0;
    ++read_index;
    for (uint8_t i = 1; i < code; ++i) {
      if (read_index >= len) return 0;
      out[write_index++] = in[read_index++];
    }
    if (code != 0xFF && read_index != len) {
      out[write_index++] = 0;
    }
  }
  return write_index;
}

size_t encode_framed(const RawFrame& frame, uint8_t* out) {
  if (frame.can_id > 0x7FF || frame.dlc > 8) return 0;
  uint8_t pdu[kMaxPduSize];
  const size_t pdu_len = encode_pdu(frame, pdu);
  const size_t cobs_len = cobs_encode(pdu, pdu_len, out);
  out[cobs_len] = 0x00;
  return cobs_len + 1;
}

void StreamDecoder::push_byte(uint8_t byte, FrameCallback on_frame, void* ctx) {
  if (byte == 0x00) {
    if (len_ == 0) return;  // 0x00 double, keepalive optionnel : rien à faire
    uint8_t pdu[kMaxPduSize];
    const size_t pdu_len = cobs_decode(buffer_.data(), len_, pdu);
    RawFrame frame{};
    if (pdu_len > 0 && pdu_len <= kMaxPduSize && decode_pdu(pdu, pdu_len, &frame)) {
      on_frame(frame, ctx);
    } else {
      ++dropped_count;
    }
    len_ = 0;
    return;
  }
  if (len_ >= buffer_.size()) {
    // Trame plus longue que kMaxCobsSize avant un 0x00 : forcément corrompue
    // ou hors gabarit. On l'abandonne et on se resynchronise au prochain 0x00.
    ++dropped_count;
    len_ = 0;
    return;
  }
  buffer_[len_++] = byte;
}

}  // namespace common
