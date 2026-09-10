#include "net_ws.h"

#include <cstdlib>
#include <cstring>

#include "esp_log.h"

namespace net_ws {
namespace {

constexpr const char* kTag = "net_ws";
constexpr size_t kMaxClients = 3;  // net_http conserve au moins un socket HTTP.
constexpr size_t kMaxClientPayload = common::kMaxPduSize;

struct PendingFrame {
  uint8_t pdu[common::kMaxPduSize];
  size_t len;
  httpd_handle_t server;
};

httpd_handle_t g_server = nullptr;
Authorize g_authorize = nullptr;
int g_clients[kMaxClients]{};
size_t g_client_count = 0;

void remove_client(int fd) {
  for (size_t i = 0; i < g_client_count; ++i) {
    if (g_clients[i] != fd) continue;
    g_clients[i] = g_clients[g_client_count - 1];
    --g_client_count;
    return;
  }
}

void add_client(int fd) {
  remove_client(fd);
  if (g_client_count == kMaxClients) {
    // Une quatrième connexion ne doit pas évincer silencieusement un client
    // déjà utile : le handshake est accepté puis immédiatement fermé.
    httpd_sess_trigger_close(g_server, fd);
    return;
  }
  g_clients[g_client_count++] = fd;
}

// Exécuté dans la tâche httpd. Une erreur d'envoi est le signal qu'un client
// ne suit plus : on le ferme plutôt que de retenir le protocole CAN.
void send_pending(void* arg) {
  auto* pending = static_cast<PendingFrame*>(arg);
  if (pending->server != g_server) {
    std::free(pending);
    return;
  }

  httpd_ws_frame_t frame{};
  frame.payload = pending->pdu;
  frame.len = pending->len;
  frame.type = HTTPD_WS_TYPE_BINARY;
  frame.final = true;
  for (size_t i = 0; i < g_client_count;) {
    const int fd = g_clients[i];
    if (httpd_ws_send_frame_async(g_server, fd, &frame) == ESP_OK) {
      ++i;
      continue;
    }
    ESP_LOGW(kTag, "client WS lent ou perdu (fd=%d)", fd);
    httpd_sess_trigger_close(g_server, fd);
    remove_client(fd);
  }
  std::free(pending);
}

esp_err_t ws_pre_handshake(httpd_req_t* request) {
  // ESP-IDF fait le handshake avant d'appeler le handler WS : l'authentification
  // doit donc vivre dans ce callback, sinon la connexion serait déjà promue.
  return g_authorize != nullptr && g_authorize(request) ? ESP_OK : ESP_FAIL;
}

esp_err_t ws_post_handshake(httpd_req_t* request) {
  add_client(httpd_req_to_sockfd(request));
  return ESP_OK;
}

esp_err_t ws_handler(httpd_req_t* request) {

  // Le WS est un miroir, jamais un pont de commande. Lire au plus un PDU pour
  // vider proprement un paquet parasite, puis fermer la session.
  httpd_ws_frame_t incoming{};
  if (httpd_ws_recv_frame(request, &incoming, 0) != ESP_OK ||
      incoming.len > kMaxClientPayload) {
    remove_client(httpd_req_to_sockfd(request));
    httpd_sess_trigger_close(g_server, httpd_req_to_sockfd(request));
    return ESP_FAIL;
  }
  uint8_t ignored[kMaxClientPayload];
  incoming.payload = ignored;
  if (incoming.len > 0) httpd_ws_recv_frame(request, &incoming, incoming.len);
  remove_client(httpd_req_to_sockfd(request));
  httpd_sess_trigger_close(g_server, httpd_req_to_sockfd(request));
  return ESP_OK;
}

}  // namespace

void start(httpd_handle_t server, Authorize authorize) {
  if (g_server != nullptr || server == nullptr) return;
  g_server = server;
  g_authorize = authorize;
  const httpd_uri_t ws{.uri = "/ws", .method = HTTP_GET, .handler = ws_handler,
                      .user_ctx = nullptr, .is_websocket = true,
                      .handle_ws_control_frames = false, .supported_subprotocol = nullptr,
                      .ws_pre_handshake_cb = ws_pre_handshake,
                      .ws_post_handshake_cb = ws_post_handshake};
  if (httpd_register_uri_handler(g_server, &ws) != ESP_OK) {
    ESP_LOGE(kTag, "enregistrement de /ws impossible");
    g_server = nullptr;
    g_authorize = nullptr;
  }
}

void stop() {
  g_client_count = 0;
  g_authorize = nullptr;
  g_server = nullptr;
}

void publish(const common::RawFrame& frame) {
  if (g_server == nullptr || g_client_count == 0) return;
  auto* pending = static_cast<PendingFrame*>(std::malloc(sizeof(PendingFrame)));
  if (pending == nullptr) return;
  pending->len = common::encode_pdu(frame, pending->pdu);
  pending->server = g_server;
  if (pending->len == 0 || httpd_queue_work(pending->server, send_pending, pending) != ESP_OK) {
    std::free(pending);
  }
}

}  // namespace net_ws
