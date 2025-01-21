### PoC for a deadlock on esp32-c3 using embassy

To build, flash and monitor the output do:

```bash
ESP_LOG=trace SSID="[SSID]" PASSWORD="[PASSWORD]" cargo run --release
```

To trigger the deadlock, run the following command in 4 terminals for about 10-20 seconds:

```bash
nc -vv [IP ADDRESS OF DEVICE] 1337 < /dev/urandom >/dev/null
```

The last thing on the trace should be something along the lines of:

```
TRACE - mutex_unlock TRACE - timer_arm_us 3fcbe298 current: 18174529 ticks: 50000 repeat: false
TRACE - timer disarm
TRACE - timer_disarm 3fcbe298
TRACE - timer_arm_us 3fcbe298 current: 18175172 ticks: 50000 repeat: false
TRACE - Dropping EspWifiPacketBuffer, freeing memory
TRACE - free 0x3fc96e44
0x3fc96e44 - wifi_echo_server::____embassy_main_task::{{closure}}::HEAP
    at ??:??
TRACE - mutex_lock ptr = 0x3fc8b38c
0x3fc8b38c - wifi_echo_server::____embassy_main_task::{{closure}}::HEAP
    at ??:??
```
