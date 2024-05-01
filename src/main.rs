//! This example shows the ease of debouncing a button with async rust.
//! Hook up a button or switch between pin 9 and ground.

#![no_std]
#![no_main]

use defmt::info;

use embassy_executor::Spawner;
use embassy_rp::gpio::{AnyPin, Level, Input, Output, Pin, Pull};
use embassy_time::{with_deadline, Duration, Instant, Timer};
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{InterruptHandler, Pio};

pub mod ws2812;
use crate::ws2812::Ws2812;

pub mod debounce;
use crate::debounce::Debouncer;

use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

// ================================================================================

#[embassy_executor::task(pool_size = 4)]
async fn read_button(btn_pin: AnyPin, led_pin: AnyPin) {
    let mut btn = Debouncer::new(Input::new(btn_pin, Pull::Up), Duration::from_millis(20));
    let mut led = Output::new(led_pin, Level::Low);

    loop {
        // button pressed
        btn.debounce().await;
        let start = Instant::now();
        info!("Button Press");

        match with_deadline(start + Duration::from_secs(1), btn.debounce()).await {
            // Button Released < 1s
            Ok(_) => {
                info!("Button pressed for: {}ms", start.elapsed().as_millis());

		led.set_high();
		Timer::after_millis(500).await;
		led.set_low();
                continue;
            }
            // button held for > 1s
            Err(_) => {
                info!("Button Held");

		led.set_high();
		Timer::after_millis(500).await;
		led.set_low();
            }
        }

        match with_deadline(start + Duration::from_secs(5), btn.debounce()).await {
            // Button released <5s
            Ok(_) => {
                info!("Button pressed for: {}ms", start.elapsed().as_millis());

		led.set_high();
		Timer::after_millis(500).await;
		led.set_low();
                continue;
            }
            // button held for > >5s
            Err(_) => {
                info!("Button Long Held");

		led.set_high();
		Timer::after_millis(500).await;
		led.set_low();
            }
        }

        // wait for button release before handling another press
        btn.debounce().await;
        info!("Button pressed for: {}ms", start.elapsed().as_millis());
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Start");

    let p = embassy_rp::init(Default::default());

    // =====
    // Initialize the NeoPixel LED.
    let Pio { mut common, sm0, .. } = Pio::new(p.PIO0, Irqs);
    let mut ws2812 = Ws2812::new(&mut common, sm0, p.DMA_CH0, p.PIN_15);

    // Spawn off one button reader per button.
    spawner.spawn(read_button(p.PIN_2.degrade(), p.PIN_6.degrade())).unwrap(); // P
    spawner.spawn(read_button(p.PIN_3.degrade(), p.PIN_7.degrade())).unwrap(); // N
    spawner.spawn(read_button(p.PIN_4.degrade(), p.PIN_8.degrade())).unwrap(); // R
    spawner.spawn(read_button(p.PIN_5.degrade(), p.PIN_9.degrade())).unwrap(); // D

    // =====
    info!("Debounce Demo");
    loop {
	// Set the NeoPixel BLUE.
	ws2812.write(&[(0,0,255).into()]).await;
	Timer::after_secs(1).await;

	// Turn off the NeoPixel
	ws2812.write(&[(0,0,0).into()]).await;
	Timer::after_secs(1).await;
    }
}
