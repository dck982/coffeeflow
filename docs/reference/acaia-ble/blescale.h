#ifndef CTRACKER_BLE_H_
#define CTRACKER_BLE_H_

#include <cstdint>
#include <memory>
#include "BLEAdvertisedDevice.h"
#include "BLEClient.h"
#include "BLERemoteCharacteristic.h"

class BLEScaleCallback {
public:
	virtual ~BLEScaleCallback() {}
	virtual void provideWeight(float weight);
    virtual void provideTime(uint32_t time);
    virtual void provideTimerState(bool timerIsOn);
};

class BLEScale {
private:
    int scantime;
    const char *serviceUUID;
    const char *charUUID;
    std::shared_ptr<BLEScaleCallback> callback; 

    bool connected;
    bool scaleDevFound = false;
    bool scanning = false;
    BLEAddress *scaleDevAddress;
    esp_ble_addr_type_t scaleDevAddressType;
    std::unique_ptr<BLEAdvertisedDeviceCallbacks> advCallbacks;
    BLEClient *bleClient;
    BLERemoteCharacteristic *bleCharacteristic;
    uint8_t msgBuf[128];
    size_t  msgBufLen;
    unsigned long last_heartbeat;

public:
    BLEScale(int scantime, const char *serviceUUID, const char *charUUID, std::shared_ptr<BLEScaleCallback> callback);
    ~BLEScale() = default;

    void scan();
    bool connect(notify_callback notifyCallback);
    void provideData(uint8_t *pData, size_t length);
    void loop();
    void disconnect();
    void gotRemoteDisconnection();
    void deviceDiscovered(BLEAdvertisedDevice &dev);
    void scanCompleted();

    bool isDeviceFound();
    bool isConnected();
};

#endif CTRACKER_BLE_H_
