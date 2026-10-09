// Génère les vecteurs de référence de `protocol` avec le code C++ archivé.
// Les fichiers produits sont versionnés ; ce programme ne sert qu'à les
// régénérer si le C++ archivé change (il ne devrait plus).
//
// Depuis la racine du dépôt :
//   c++ -std=c++17 -Wall -Wextra -I archive/firmware/common/include \
//     archive/firmware/common/src/crc.cpp archive/firmware/common/src/framing.cpp \
//     firmware/rust/protocol/vectors/golden.cpp -o /tmp/protocol_golden
//   /tmp/protocol_golden firmware/rust/protocol/vectors
//
// Écrit `encoded.txt` (trames, PDU cadrés, SCACE, CRC, COBS, flux) et
// `differential.txt` (accepté ou refusé par unpack(), puis pack() de la
// valeur relue).

#include <cstdint>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

#include "common/crc.hpp"
#include "common/framing.hpp"
#include "common/messages.hpp"
#include "common/protocol.hpp"
#include "common/scace_ble.hpp"
#include "common/version.hpp"

using namespace common;

namespace {

std::string hex(const uint8_t* data, size_t len) {
  if (len == 0) return "-";
  std::string s;
  char buf[3];
  for (size_t i = 0; i < len; ++i) {
    std::snprintf(buf, sizeof(buf), "%02x", data[i]);
    s += buf;
  }
  return s;
}

std::string hex(const std::vector<uint8_t>& v) { return hex(v.data(), v.size()); }

FILE* g_enc = nullptr;

void emit_frame(const char* name, MessageType type, Dest dest, Node src, const Frame& data, uint8_t dlc) {
  RawFrame raw;
  raw.can_id = encode_can_id({type, dest, src});
  raw.dlc = dlc;
  raw.data = data;
  uint8_t framed[kMaxCobsSize + 1];
  const size_t framed_len = encode_framed(raw, framed);
  std::fprintf(g_enc, "frame %s %03x %u %s %s\n", name, raw.can_id, dlc,
               hex(data.data(), 8).c_str(), hex(framed, framed_len).c_str());
}

// Octets bruts d'un message sans classe de charge utile (CONFIRM, FLASH_DATA).
Frame raw_bytes(std::initializer_list<uint8_t> bytes) {
  Frame f{};
  size_t i = 0;
  for (uint8_t b : bytes) f[i++] = b;
  return f;
}

void write_encoded() {
  const Dest S = Dest::kSensors, Sc = Dest::kScreen, B = Dest::kBroadcast;
  const Node Screen = Node::kScreen, Sensors = Node::kSensors;
  using T = MessageType;

  emit_frame("stop", T::kStop, S, Screen, Frame{}, 0);
  emit_frame("reset", T::kReset, S, Screen, Frame{}, 0);
  emit_frame("set_42_500", T::kSet, S, Screen, SetPayload{42, 500}.pack(), 3);
  emit_frame("set_100_default", T::kSet, S, Screen, SetPayload{100, 0}.pack(), 3);
  emit_frame("set_0_65535", T::kSet, S, Screen, SetPayload{0, 65535}.pack(), 3);
  emit_frame("dimmer_reset", T::kDimmerCommand, S, Screen, DimmerCommandPayload{DimmerCommand::kReset}.pack(), 1);
  emit_frame("dimmer_recalibrate", T::kDimmerCommand, S, Screen,
             DimmerCommandPayload{DimmerCommand::kRecalibrate}.pack(), 1);
  emit_frame("set_heating_on", T::kSetHeating, S, Screen, SetHeatingPayload{true, 1000}.pack(), 3);
  emit_frame("set_heating_off", T::kSetHeating, S, Screen, SetHeatingPayload{false, 0}.pack(), 3);
  emit_frame("confirm_ota", T::kConfirmSensorsOta, S, Screen, raw_bytes({kHeatingProtocolConfirmationToken}), 1);
  {
    SetHeatingPowerPayload p{600, 1500, false};
    emit_frame("set_heating_power_dlc4", T::kSetHeatingPower, S, Screen, p.pack(), static_cast<uint8_t>(p.length()));
    SetHeatingPowerPayload q{900, 1500, true};
    emit_frame("set_heating_power_dlc5", T::kSetHeatingPower, S, Screen, q.pack(), static_cast<uint8_t>(q.length()));
    SetHeatingPowerPayload z{0, 1500, false};
    emit_frame("set_heating_power_zero", T::kSetHeatingPower, S, Screen, z.pack(), static_cast<uint8_t>(z.length()));
    SetHeatingPowerPayload m{1000, 65535, true};
    emit_frame("set_heating_power_max", T::kSetHeatingPower, S, Screen, m.pack(), static_cast<uint8_t>(m.length()));
  }
  emit_frame("maintenance_open_max", T::kMaintenanceValve, S, Screen,
             MaintenanceValvePayload{true, kMaintenanceValveMaxMs}.pack(), 3);
  emit_frame("maintenance_open_1", T::kMaintenanceValve, S, Screen, MaintenanceValvePayload{true, 1}.pack(), 3);
  emit_frame("maintenance_close", T::kMaintenanceValve, S, Screen, MaintenanceValvePayload{false, 1234}.pack(), 3);
  emit_frame("ping_legacy", T::kPing, B, Screen, Frame{}, 0);
  emit_frame("ping_screen", T::kPing, S, Screen, NodeIdentityPayload{Node::kScreen, 0, 4, 0, 3600}.pack(), 8);
  emit_frame("ping_sensors_broadcast", T::kPing, B, Sensors,
             NodeIdentityPayload{Node::kSensors, 0, 3, 48, 1}.pack(), 8);
  emit_frame("pong_sensors", T::kPong, Sc, Sensors, NodeIdentityPayload{Node::kSensors, 1, 2, 3, 123456789}.pack(), 8);
  emit_frame("pong_uptime_max", T::kPong, Sc, Sensors,
             NodeIdentityPayload{Node::kSensors, 255, 255, 255, 0xFFFFFFFFu}.pack(), 8);
  emit_frame("pong_screen", T::kPong, S, Screen, NodeIdentityPayload{Node::kScreen, 0, 4, 0, 42}.pack(), 8);
  emit_frame("reqstatus_flow", T::kReqStatus, S, Screen, ReqStatusPayload{T::kStatusFlow, 200}.pack(), 3);
  emit_frame("reqstatus_heating", T::kReqStatus, S, Screen, ReqStatusPayload{T::kStatusHeating, 250}.pack(), 3);
  emit_frame("reqstatus_stop", T::kReqStatus, S, Screen, ReqStatusPayload{T::kStatusPressure, 0}.pack(), 3);
  {
    StatusPressurePayload p;
    p.pressure_raw = 0xABCDEF;
    p.temperature_raw = 0x1234;
    p.timestamp_ms = 60000;
    p.flags = 1;
    emit_frame("status_pressure", T::kStatusPressure, Sc, Sensors, p.pack(), 8);
    StatusPressurePayload q;
    q.pressure_raw = 0x800000;
    q.temperature_raw = 0xFFFF;
    q.timestamp_ms = 0;
    q.flags = 3;
    emit_frame("status_pressure_timeout", T::kStatusPressure, Sc, Sensors, q.pack(), 8);
  }
  emit_frame("status_flow", T::kStatusFlow, Sc, Sensors, StatusFlowPayload{4000000000u, 1234, 1}.pack(), 7);
  emit_frame("status_flow_zero", T::kStatusFlow, Sc, Sensors, StatusFlowPayload{0, 0, 1}.pack(), 7);
  {
    StatusActuatorsPayload p;
    p.valve_open = true;
    p.pump_pct = 77;
    p.lease_remaining_ms = 480;
    p.continuous_on_ms = 59000;
    p.flags = 0x01 | 0x04 | StatusActuatorsPayload::kFlagMaintenanceValveCapable;
    emit_frame("status_actuators", T::kStatusActuators, Sc, Sensors, p.pack(), 7);
    StatusActuatorsPayload q;
    q.flags = StatusActuatorsPayload::kFlagMaintenanceValveCapable | StatusActuatorsPayload::kFlagMaintenanceValveOpen;
    emit_frame("status_actuators_idle", T::kStatusActuators, Sc, Sensors, q.pack(), 7);
  }
  {
    StatusHeatingPayload p;
    p.heater_on = true;
    p.lease_remaining_ms = 350;
    p.power_capable = true;
    p.fine_power_capable = true;
    p.window_restart_capable = true;
    p.power_permille = 937;
    emit_frame("status_heating_full", T::kStatusHeating, Sc, Sensors, p.pack(), 6);
    StatusHeatingPayload q;
    emit_frame("status_heating_off", T::kStatusHeating, Sc, Sensors, q.pack(), 6);
    StatusHeatingPayload r;
    r.power_capable = true;
    r.fine_power_capable = true;
    r.power_permille = 1000;
    emit_frame("status_heating_max", T::kStatusHeating, Sc, Sensors, r.pack(), 6);
    StatusHeatingPayload s;
    s.heater_on = true;
    s.lease_remaining_ms = 1000;
    s.power_capable = true;
    s.power_permille = 60;
    emit_frame("status_heating_coarse_dlc5", T::kStatusHeating, Sc, Sensors, s.pack(), 5);
    emit_frame("status_heating_basic_dlc4", T::kStatusHeating, Sc, Sensors, q.pack(), 4);
  }
  emit_frame("log_lockout", T::kLog, B, Sensors, LogPayload{6, 3, 0, 60000}.pack(), 8);
  emit_frame("log_boiler_ntc", T::kLog, B, Screen, LogPayload{43, 2, 0x48, (26485u << 16) | 1050u}.pack(), 8);
  emit_frame("log_args_max", T::kLog, B, Sensors, LogPayload{255, 255, 0xFFFF, 0xFFFFFFFFu}.pack(), 8);
  {
    FlashCtrlPayload p;
    p.subcmd = FlashSubCmd::kBegin;
    p.image_size = 1048576;
    emit_frame("flash_begin", T::kFlashCtrl, S, Screen, p.pack(), 8);
    p = FlashCtrlPayload{};
    p.subcmd = FlashSubCmd::kBlockAck;
    p.block_number = 42;
    p.block_crc16 = 0xBEEF;
    emit_frame("flash_block_ack", T::kFlashCtrl, Sc, Sensors, p.pack(), 8);
    p = FlashCtrlPayload{};
    p.subcmd = FlashSubCmd::kBlockStart;
    p.block_number = 7;
    p.block_crc16 = 0xA55A;
    emit_frame("flash_block_start", T::kFlashCtrl, S, Screen, p.pack(), 5);
    p = FlashCtrlPayload{};
    p.subcmd = FlashSubCmd::kEnd;
    p.image_crc32 = 0xDEADBEEF;
    emit_frame("flash_end", T::kFlashCtrl, S, Screen, p.pack(), 8);
    p = FlashCtrlPayload{};
    p.subcmd = FlashSubCmd::kEnd;
    emit_frame("flash_end_ack", T::kFlashCtrl, Sc, Sensors, p.pack(), 8);
    p = FlashCtrlPayload{};
    p.subcmd = FlashSubCmd::kAbort;
    emit_frame("flash_abort", T::kFlashCtrl, S, Screen, p.pack(), 8);
  }
  emit_frame("flash_data_8", T::kFlashData, S, Screen, raw_bytes({0xDE, 0xAD, 0xBE, 0xEF, 0, 1, 2, 0xFF}), 8);
  emit_frame("flash_data_3", T::kFlashData, S, Screen, raw_bytes({0, 0, 7}), 3);
  emit_frame("flash_data_zeros", T::kFlashData, S, Screen, Frame{}, 8);

  // Trame SCACE.
  struct ScaceCase {
    const char* name;
    scace::Frame frame;
  };
  const ScaceCase scace_cases[] = {
      {"ok", {0xfffe, 0x1234, 26416, -3, 2503, scace::Status::kOk}},
      {"no_measurement", {}},
      {"open", {1, 65535, -32768, 32767, scace::kNoTemperature, scace::Status::kOpen}},
      {"shorted", {0, 0, 0, 0, -1234, scace::Status::kShorted}},
  };
  for (const auto& c : scace_cases) {
    uint8_t bytes[scace::kFrameSize];
    scace::encode(c.frame, bytes);
    std::fprintf(g_enc, "scace %s %s\n", c.name, hex(bytes, sizeof(bytes)).c_str());
  }
}

uint32_t g_rng = 0x2545F491u;
uint32_t next_rand() {
  g_rng ^= g_rng << 13;
  g_rng ^= g_rng >> 17;
  g_rng ^= g_rng << 5;
  return g_rng;
}

uint8_t random_byte() {
  static const uint8_t kInteresting[] = {0x00, 0x00, 0x01, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x09, 0x0A,
                                         0x1E, 0x63, 0x64, 0x65, 0x80, 0xE8, 0xE9, 0xFF, 0xFF, 0x10, 0x20};
  if (next_rand() % 3 == 0) return static_cast<uint8_t>(next_rand());
  return kInteresting[next_rand() % sizeof(kInteresting)];
}

template <typename P>
void diff_case(FILE* f, uint8_t type, const uint8_t* in, size_t len) {
  P payload{};
  const bool ok = P::unpack(in, len, &payload);
  std::fprintf(f, "%02x %zu %s %s\n", type, len, hex(in, len).c_str(),
               ok ? hex(payload.pack().data(), 8).c_str() : "REJECT");
}

void write_differential(FILE* f) {
  for (int round = 0; round < 3; ++round) {
    for (size_t len = 0; len <= 8; ++len) {
      for (int sample = 0; sample < 14; ++sample) {
        uint8_t in[8] = {};
        if (round == 1 && sample == 0) {
          std::memset(in, 0, sizeof(in));
        } else if (round == 1 && sample == 1) {
          std::memset(in, 0xFF, sizeof(in));
        } else {
          for (auto& b : in) b = random_byte();
        }
        diff_case<SetPayload>(f, 0x01, in, len);
        diff_case<DimmerCommandPayload>(f, 0x03, in, len);
        diff_case<SetHeatingPayload>(f, 0x04, in, len);
        diff_case<SetHeatingPowerPayload>(f, 0x06, in, len);
        diff_case<MaintenanceValvePayload>(f, 0x07, in, len);
        diff_case<NodeIdentityPayload>(f, 0x08, in, len);
        diff_case<NodeIdentityPayload>(f, 0x09, in, len);
        diff_case<ReqStatusPayload>(f, 0x10, in, len);
        diff_case<StatusPressurePayload>(f, 0x20, in, len);
        diff_case<StatusFlowPayload>(f, 0x21, in, len);
        diff_case<StatusActuatorsPayload>(f, 0x22, in, len);
        diff_case<StatusHeatingPayload>(f, 0x23, in, len);
        diff_case<LogPayload>(f, 0x30, in, len);
        diff_case<FlashCtrlPayload>(f, 0x38, in, len);
      }
    }
  }
  // Cas ciblés pour SET_HEATING_POWER et STATUS_HEATING (drapeaux, bornes).
  const uint8_t power_cases[][5] = {
      {100, 0xDC, 0x05, 0, 0}, {101, 0xDC, 0x05, 0, 0}, {0xE8, 0x03, 0xDC, 0x05, 1},
      {0xE9, 0x03, 0xDC, 0x05, 0}, {0, 0, 0xDC, 0x05, 2}, {0, 0, 0xDC, 0x05, 0x81},
  };
  for (const auto& c : power_cases) {
    for (size_t len = 3; len <= 5; ++len) diff_case<SetHeatingPowerPayload>(f, 0x06, c, len);
  }
  for (uint8_t caps = 0; caps < 16; ++caps) {
    for (uint8_t hi : {0, 60, 100, 101}) {
      for (uint8_t lo : {0, 5, 9, 10}) {
        const uint8_t in[6] = {1, 0x5E, 0x01, caps, hi, lo};
        for (size_t len = 3; len <= 7 && len <= 6; ++len) diff_case<StatusHeatingPayload>(f, 0x23, in, len);
      }
    }
  }
}

void write_crc_cobs_stream() {
  // CRC sur quelques entrées.
  const char* strings[] = {"", "a", "123456789", "coffeeflow"};
  for (const char* s : strings) {
    const auto* d = reinterpret_cast<const uint8_t*>(s);
    std::fprintf(g_enc, "crc %s %04x %08x\n", hex(d, std::strlen(s)).c_str(), crc16_ccitt(d, std::strlen(s)),
                 crc32_ieee(d, std::strlen(s)));
  }
  std::vector<uint8_t> big(2048);
  for (size_t i = 0; i < big.size(); ++i) big[i] = static_cast<uint8_t>(i * 7 + (i >> 8));
  std::fprintf(g_enc, "crc %s %04x %08x\n", hex(big).c_str(), crc16_ccitt(big.data(), big.size()),
               crc32_ieee(big.data(), big.size()));

  // COBS : encodage d'entrées de formes variées, y compris le bloc de 254 octets.
  std::vector<std::vector<uint8_t>> inputs;
  inputs.push_back({});
  inputs.push_back({0});
  inputs.push_back({0, 0});
  inputs.push_back({1, 2, 3});
  inputs.push_back({0, 1, 0, 2, 0});
  inputs.push_back(std::vector<uint8_t>(13, 0));
  inputs.push_back(std::vector<uint8_t>(253, 7));
  inputs.push_back(std::vector<uint8_t>(254, 7));
  inputs.push_back(std::vector<uint8_t>(255, 7));
  inputs.push_back(std::vector<uint8_t>(300, 9));
  inputs.push_back(std::vector<uint8_t>(508, 1));
  {
    std::vector<uint8_t> v(300, 5);
    v[100] = 0;
    v[299] = 0;
    inputs.push_back(v);
  }
  for (int i = 0; i < 24; ++i) {
    std::vector<uint8_t> v(next_rand() % 40);
    for (auto& b : v) b = (next_rand() % 4 == 0) ? 0 : random_byte();
    inputs.push_back(v);
  }
  for (const auto& in : inputs) {
    std::vector<uint8_t> out(in.size() + in.size() / 254 + 2);
    const size_t n = cobs_encode(in.data(), in.size(), out.data());
    std::fprintf(g_enc, "cobs %s %s\n", hex(in).c_str(), hex(out.data(), n).c_str());
  }
  // COBS inverse sur des entrées quelconques (0 = refus côté C++).
  for (int i = 0; i < 80; ++i) {
    std::vector<uint8_t> v(1 + next_rand() % 16);
    for (auto& b : v) b = (next_rand() % 6 == 0) ? 0 : static_cast<uint8_t>(1 + next_rand() % 12);
    std::vector<uint8_t> out(v.size() + 1);
    const size_t n = cobs_decode(v.data(), v.size(), out.data());
    std::fprintf(g_enc, "uncobs %s %zu %s\n", hex(v).c_str(), n, hex(out.data(), n).c_str());
  }

  // Flux : trames valides entrecoupées de bruit, pour le resynchronisation.
  std::vector<std::vector<uint8_t>> good;
  {
    const RawFrame frames[] = {
        {encode_can_id({MessageType::kPing, Dest::kBroadcast, Node::kScreen}), 0, {}},
        {encode_can_id({MessageType::kSet, Dest::kSensors, Node::kScreen}), 3, {42, 0xF4, 0x01}},
        {encode_can_id({MessageType::kFlashData, Dest::kSensors, Node::kScreen}), 8, {0, 0, 0, 0, 0, 0, 0, 0}},
        {encode_can_id({MessageType::kStatusFlow, Dest::kScreen, Node::kSensors}), 7, {1, 2, 3, 4, 5, 6, 7}},
    };
    for (const auto& fr : frames) {
      uint8_t framed[kMaxCobsSize + 1];
      const size_t n = encode_framed(fr, framed);
      good.emplace_back(framed, framed + n);
    }
  }
  for (int i = 0; i < 40; ++i) {
    std::vector<uint8_t> stream;
    const int pieces = 1 + next_rand() % 5;
    for (int p = 0; p < pieces; ++p) {
      switch (next_rand() % 6) {
        case 0:
        case 1:
        case 2: {
          auto g = good[next_rand() % good.size()];
          if (next_rand() % 3 == 0) g[next_rand() % (g.size() - 1)] ^= static_cast<uint8_t>(1 + next_rand() % 255);
          stream.insert(stream.end(), g.begin(), g.end());
          break;
        }
        case 3: {
          const size_t n = next_rand() % 24;
          for (size_t k = 0; k < n; ++k) stream.push_back(static_cast<uint8_t>(next_rand()));
          break;
        }
        case 4:
          stream.push_back(0);
          stream.push_back(0);
          break;
        default: {
          const size_t n = 10 + next_rand() % 30;
          for (size_t k = 0; k < n; ++k) stream.push_back(static_cast<uint8_t>(1 + next_rand() % 255));
          break;
        }
      }
    }
    struct Capture {
      std::string frames;
    } capture;
    StreamDecoder decoder;
    for (uint8_t b : stream) {
      decoder.push_byte(
          b,
          [](const RawFrame& fr, void* ctx) {
            auto* c = static_cast<Capture*>(ctx);
            char buf[64];
            std::snprintf(buf, sizeof(buf), " %03x:%u:", fr.can_id, fr.dlc);
            c->frames += buf;
            c->frames += hex(fr.data.data(), fr.dlc);
          },
          &capture);
    }
    std::fprintf(g_enc, "stream %s %u%s\n", hex(stream).c_str(), decoder.dropped_count, capture.frames.c_str());
  }
}

}  // namespace

int main(int argc, char** argv) {
  if (argc != 2) {
    std::fprintf(stderr, "usage: %s <dossier de sortie>\n", argv[0]);
    return 2;
  }
  const std::string dir = argv[1];
  g_enc = std::fopen((dir + "/encoded.txt").c_str(), "w");
  FILE* diff = std::fopen((dir + "/differential.txt").c_str(), "w");
  if (!g_enc || !diff) {
    std::fprintf(stderr, "ouverture des fichiers de sortie impossible\n");
    return 1;
  }
  std::fputs("# Généré par golden.cpp à partir de archive/firmware/common. Ne pas éditer.\n"
             "# frame <nom> <id hex> <dlc> <8 octets> <trame cadrée COBS+00>\n"
             "# scace <nom> <12 octets>\n# crc <entrée> <crc16> <crc32>\n"
             "# cobs <entrée> <sortie>\n# uncobs <entrée> <longueur décodée> <sortie>\n"
             "# stream <octets> <dropped_count> [<id>:<dlc>:<données>]...\n",
             g_enc);
  std::fputs("# Généré par golden.cpp à partir de archive/firmware/common. Ne pas éditer.\n"
             "# <type hex> <len> <entrée> <pack() après unpack(), ou REJECT>\n",
             diff);
  write_encoded();
  write_crc_cobs_stream();
  write_differential(diff);
  std::fclose(g_enc);
  std::fclose(diff);
  return 0;
}
