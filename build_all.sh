set -e

cargo +nightly build --release --features="module-button, f051c8u, can-mcp25625, can-printstat, log-text-rtt, log-level-debug" --color=always
arm-none-eabi-objcopy -O binary ./target/thumbv6m-none-eabi/release/vhrd-module-template ./target/thumbv6m-none-eabi/release/button_f051c8u.bin

cargo +nightly build --release --features="module-afe, module-afe-hx711, f072c8u, can-stm, log-text-rtt, log-level-debug" --color=always
arm-none-eabi-objcopy -O binary ./target/thumbv6m-none-eabi/release/vhrd-module-template ./target/thumbv6m-none-eabi/release/afe_hx711_f072c8u.bin

cargo +nightly build --release --features="module-led, f072c8u, can-stm, can-printstat, log-text-rtt, log-level-debug" --color=always
arm-none-eabi-objcopy -O binary ./target/thumbv6m-none-eabi/release/vhrd-module-template ./target/thumbv6m-none-eabi/release/led_f072c8u.bin

cargo +nightly build --release --features="module-pi, f072c8u, can-stm, can-printstat, log-text-rtt, log-level-debug" --color=always
arm-none-eabi-objcopy -O binary ./target/thumbv6m-none-eabi/release/vhrd-module-template ./target/thumbv6m-none-eabi/release/pi_f072c8u.bin

rsync -avz ./target/thumbv6m-none-eabi/release/*.bin pi@192.168.0.23:/home/pi/module-firmwares/
