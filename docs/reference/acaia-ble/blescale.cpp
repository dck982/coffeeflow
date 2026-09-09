
#include "blescale.h"
#include <Arduino.h>
#include "BLEDevice.h"

#define MSG_PREFIX1 (0xEF)
#define MSG_PREFIX2 (0xDD)

static void serialPrintHexBuffer(uint8_t *buf, size_t buflen) {
    for (int i = 0; i < buflen; i++) {
      if (buf[i]<16) {
        Serial.print("0");
      }
      Serial.print(buf[i],HEX);
      Serial.print(" ");
    }
}

static void processSettings(uint8_t *payload, BLEScaleCallback *cb) {
    bool timerMode = (payload[0]&0x80)!=0;
    if (cb) {
        cb->provideTimerState(timerMode);
    }
    // payload[2] is scale mode
    // 3 is Flow + Auto-Tare    
    // enum scalemode_t {
    // WEIGHT_ONLY=0, WEIGHT_TIME, FLOW, FLOW_TARE, AUTO_START, AUTO_TARE, SPECIAL
    // };
}

static float decodeWeight(uint8_t *msg) {
    uint32_t value = ((uint32_t)msg[0]) + (((uint32_t)msg[1])<<8) + (((uint32_t)msg[2])<<16) + (((uint32_t)msg[3])<<24);
    float fval = (float)value;
    switch (msg[4]) {
        case 1: fval /= 10.0; break;
        case 2: fval /= 100.0; break;
        case 3: fval /= 1000.0; break;
        case 4: fval /= 10000.0; break;
    }
    if ((msg[5]&2)==2) {
        fval *= -1.0;
    }
    return fval;
}
static uint32_t decodeTime(uint8_t *payload) {
    uint32_t decoded_scale_time = ((uint32_t)payload[0])*6000 + ((uint32_t)payload[1])*100 + ((uint32_t)payload[2])*10;
    if (decoded_scale_time<=20) {
        decoded_scale_time = 0;
    }
    return decoded_scale_time;
}

enum button_t {
  TARE, START, STOP, RESET, UNKNOWN
};

static size_t processMessage(uint8_t msgType, uint8_t *msg, size_t msgLen, BLEScaleCallback *cb) {
    switch (msgType) {
        case 5:
            if (msgLen>=6) {
                if (cb) {
                    cb->provideWeight(decodeWeight(msg));
                }
                return 6;
            }
            break;
        case 7:
            if (msgLen>=3) {
                if (cb) {
                    cb->provideTime(decodeTime(msg));
                }
                return 3;
            }
            break;
        case 8:
            if (msgLen>0) {
                uint8_t btn = msg[0];
                button_t btnType = UNKNOWN;
                switch (btn) {
                case 0:
                    btnType = TARE;
                    break;
                case 8:
                    btnType = START;
                    break;
                case 9:
                    btnType = RESET;
                    break;
                case 10:
                    btnType = STOP;
                    break;
                }
                //Serial.println("BUTTON "+String(btnType));
                return 1;
            }
            break;

        default:
            Serial.print("Incoming message "+String(msgType)+" : ");
            serialPrintHexBuffer(msg,msgLen);
            Serial.println("");
    }
    return msgLen;
}


static void processNotification(uint8_t cmd,uint8_t *payload,size_t payloadLen, BLEScaleCallback *cb) {
    switch (cmd) {
        case 8:
            if (payloadLen>2) {
                processSettings(payload,cb);
            }
            break;
        case 12: 
            //Serial.println("Process notification "+String(cmd)+", payloadLen="+String(payloadLen));
            if (payloadLen>1) {
                size_t pLen = payloadLen;
                size_t ofs = 0;
                while (pLen>0) {
                    size_t consumed = processMessage(payload[ofs],payload+ofs+1,pLen-1,cb) + 1;
                    //Serial.println("ofs="+String(ofs)+",pLen="+String(pLen)+",msg="+String(payload[ofs])+",consumed="+String(consumed));
                    pLen -= consumed;
                    ofs += consumed;
                }
            }
            break;
        default:
            Serial.println("New notification, cmd="+String(cmd)+", payload "+String(payloadLen)+" bytes");
    }
}

static void send_fixedlen_payload(BLERemoteCharacteristic *bleChar, uint8_t cmd, uint8_t *buf, size_t buflen) {
    uint8_t msg[64];
    if (buflen+4>=64) {
        Serial.println("Outgoing message too large");
        return;
    }
    msg[0] = MSG_PREFIX1;
    msg[1] = MSG_PREFIX2;
    msg[2] = cmd;
    memmove(msg+3,buf,buflen);
    uint8_t cksum1 = 0;
    uint8_t cksum2 = 0;
    for (int i = 0; i < buflen; i++) {
        if (i%2==0) {
            cksum1 += msg[3+i];
        } else {
            cksum2 += msg[3+i];
        }
    }
    msg[3+buflen] = cksum1;
    msg[4+buflen] = cksum2;
    if (bleChar) {
        bleChar->writeValue(msg,buflen+5);
    }
}

static void send_payload(BLERemoteCharacteristic *bleChar, uint8_t cmd, uint8_t *buf, size_t buflen) {
    uint8_t msg[64];
    if (buflen+1>=64) {
        Serial.println("Outgoing message too large");
        return;
    }
    msg[0] = buflen+1;
    memmove(msg+1,buf,buflen);
    send_fixedlen_payload(bleChar,cmd,msg,buflen+1);
}

static void send_heartbeat(BLERemoteCharacteristic *bleChar) {
    // id
    uint8_t payloadA[15] = {0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0x31, 0x32, 0x33, 0x34};
    send_fixedlen_payload(bleChar,0x0b,payloadA,sizeof(payloadA));
    // heartbeat
    uint8_t payloadB[1] = {0};
    send_payload(bleChar,0,payloadB,(size_t)sizeof(payloadB));
}

static void send_request_notifications(BLERemoteCharacteristic *bleChar) {
    uint8_t payload[8] = {0x00, 0x01, 0x01, 0x02, 0x02, 0x05, 0x03, 0x04};
    send_payload(bleChar,0xc,payload,(size_t)sizeof(payload));
}

BLEScale::BLEScale(int scantime, const char *serviceUUID, const char *charUUID, std::shared_ptr<BLEScaleCallback> callback)
    : scantime(scantime), serviceUUID(serviceUUID), charUUID(charUUID), callback(callback) {
        BLEDevice::init("");
    }

// The BLE scan-complete callback is a plain C function pointer, so it routes
// through this single active-scale pointer. There is only ever one BLEScale,
// mirroring the global scale instance and notifyCallback in coffeetracker.ino.
static BLEScale *g_scanOwner = nullptr;

static void scanCompleteCB(BLEScanResults results) {
    if (g_scanOwner) {
        g_scanOwner->scanCompleted();
    }
}

class MyAdvertisedDeviceCallbacks: public BLEAdvertisedDeviceCallbacks {
    private:
        BLEScale *scale;
        BLEUUID serviceUUID;

    public:
        MyAdvertisedDeviceCallbacks(BLEScale *scale, BLEUUID svcUUID)
            : scale(scale), serviceUUID(svcUUID) {}

    void onResult(BLEAdvertisedDevice advertisedDevice) {
      if (advertisedDevice.haveServiceUUID() && advertisedDevice.isAdvertisingService(serviceUUID)) {
        Serial.println("Found ACAIA scale");
        scale->deviceDiscovered(advertisedDevice);
        BLEDevice::getScan()->stop();
      }
    }
};

void BLEScale::scan() {
    if (scanning || scaleDevFound) {
        // Either a scan is already running asynchronously, or one has found a
        // device that hasn't been consumed by connect() yet. In both cases we
        // must not restart here (a restart would clear scaleDevFound before the
        // caller gets to act on it). Returning immediately also keeps loop()
        // free to poll the buttons. onResult()/scanCompleted() drive the state.
        return;
    }

    scaleDevFound = false;
    scaleDevAddress = nullptr;

    BLEScan* pBLEScan = BLEDevice::getScan();
    if (!advCallbacks) {
        advCallbacks.reset(new MyAdvertisedDeviceCallbacks(this, BLEUUID(serviceUUID)));
    }
    pBLEScan->setAdvertisedDeviceCallbacks(advCallbacks.get());
    pBLEScan->setActiveScan(true); //active scan uses more power, but get results faster
    pBLEScan->setInterval(100);
    pBLEScan->setWindow(99);  // less or equal setInterval value
    pBLEScan->clearResults();  // release memory from any previous scan

    g_scanOwner = this;
    scanning = true;
    Serial.println("BLE Scan...");
    pBLEScan->start(scantime, scanCompleteCB, false);  // non-blocking, returns immediately
}

void BLEScale::deviceDiscovered(BLEAdvertisedDevice &dev) {
    scaleDevAddress = new BLEAddress(*dev.getAddress().getNative());
    scaleDevAddressType = dev.getAddressType();
    scaleDevFound = true;
    scanning = false;
}

void BLEScale::scanCompleted() {
    scanning = false;
}

bool BLEScale::isDeviceFound() {
    return scaleDevFound;
}

bool BLEScale::isConnected() {
    return connected;
}

class MyClientCallback : public BLEClientCallbacks {
    BLEScale *scale;

public:
    MyClientCallback(BLEScale *bleScale) {
        scale = bleScale;
    }
    ~MyClientCallback() = default;

    void onConnect(BLEClient* pclient) {}

    void onDisconnect(BLEClient* pclient) {
        scale->gotRemoteDisconnection();
    }
};

void BLEScale::gotRemoteDisconnection() {
    connected = false;
}

void BLEScale::provideData(uint8_t *pData, size_t length) {
    if (msgBufLen+length<sizeof(msgBuf)) {
        memcpy(msgBuf+msgBufLen,pData,length);
        msgBufLen += length;
    }
}

bool BLEScale::connect(notify_callback notifyCallback) {
    msgBufLen = 0;
    scaleDevFound = false;  // consume the discovery so a later return to SCAN rescans

    bleClient = BLEDevice::createClient();

    bleClient->setClientCallbacks(new MyClientCallback(this));
    bleClient->connect(*scaleDevAddress,scaleDevAddressType); 
    bleClient->setMTU(517); 

    BLERemoteService* pRemoteService = bleClient->getService(serviceUUID);
    if (pRemoteService == nullptr) {
        return false;
    }

    BLERemoteCharacteristic *remoteChar = pRemoteService->getCharacteristic(charUUID);
    if (remoteChar == nullptr) {
        return false;
    }
    bleCharacteristic = remoteChar;

    if(bleCharacteristic->canNotify()) {
        bleCharacteristic->registerForNotify(notifyCallback);
    }

    connected = true;
    return true;
}

void BLEScale::disconnect() {
    connected = false;
    msgBufLen = 0;
    if (bleCharacteristic) {
        bleCharacteristic->registerForNotify(nullptr);   
        bleCharacteristic = nullptr;     
    }
    if (bleClient) {
        bleClient->setClientCallbacks(nullptr);
        bleClient->disconnect();
        bleClient = nullptr;
    }
}

void BLEScale::loop() {
    uint8_t message[64];

    // Power off message
    if (msgBufLen>=3) {
        if (msgBuf[0]==MSG_PREFIX1 && msgBuf[1]==MSG_PREFIX2 && msgBuf[2]==0x20) {
            gotRemoteDisconnection();
            return;
        }
    }

    // Normal message
    if (msgBufLen>=6) {
        if (msgBuf[0]==MSG_PREFIX1 && msgBuf[1]==MSG_PREFIX2) {
            uint8_t cmd = msgBuf[2];
            size_t payloadLen = msgBuf[3];
            //Serial.println("in payloadLen="+String(payloadLen));
            if (payloadLen==0) {
                Serial.println("Invalid packet");
                msgBufLen = 0;
                return;
            }
            if (msgBufLen<payloadLen+5) {
                // need more data
                return;
            }
#if 0
            Serial.print("Incoming data: ");
            serialPrintHexBuffer(msgBuf,msgBufLen);
            Serial.println("");
#endif
            if (payloadLen>sizeof(message)) {
                memmove(message,msgBuf+4,sizeof(message));
            } else {
                memmove(message,msgBuf+4,payloadLen-1);
            }
            msgBufLen -= payloadLen+5;
            if (msgBufLen>0) {
                memmove(msgBuf,msgBuf+payloadLen+5,msgBufLen);
            }
#if 0
            Serial.print("Message: ");
            serialPrintHexBuffer(message,payloadLen-1);
            Serial.println("");
#endif
            processNotification(cmd,message,payloadLen-1,callback.get());
        } else {
            Serial.println("Could not find a valid message prefix!");
            // we've got a problem, try to slide by 1
            memmove(msgBuf,msgBuf+1,msgBufLen-1);
            msgBufLen--;
        }
    }

    if (millis()-last_heartbeat>2500) {
        //Serial.println("Sending HB");
        send_heartbeat(bleCharacteristic);
        send_request_notifications(bleCharacteristic);
        last_heartbeat = millis();
    }
}

