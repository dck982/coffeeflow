// Serveur HTTP lot 5. Client du cœur : n'inclut que core/core.h côté machine.
// Le provisioning AP reste dans net_wifi et ne demande pas le secret HTTP.
#pragma once

namespace net_http {
void start();
void stop();
}  // namespace net_http
