//! This example shows the ease of debouncing a button with async rust.
//! Hook up a button or switch between pin 9 and ground.

#![no_std]
#![no_main]

use defmt::info;

use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Input, Output, Pull};
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_time::{with_deadline, Duration, Instant, Timer};

pub mod ws2812;
use crate::ws2812::Ws2812;

use {defmt_rtt as _, panic_probe as _};

// ================================================================================

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

pub struct Debouncer<'a> {
    input: Input<'a>,
    debounce: Duration,
}

impl<'a> Debouncer<'a> {
    pub fn new(input: Input<'a>, debounce: Duration) -> Self {
        Self { input, debounce }
    }

    pub async fn debounce(&mut self) -> Level {
        loop {
            let l1 = self.input.get_level();

            self.input.wait_for_any_edge().await;

            Timer::after(self.debounce).await;

            let l2 = self.input.get_level();
            if l1 != l2 {
                break l2;
            }
        }
    }
}

// ================================================================================

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Start");

    let p = embassy_rp::init(Default::default());

    // Initialize the button pin.
    let mut btn = Debouncer::new(Input::new(p.PIN_5, Pull::Up), Duration::from_millis(10));

    // Initialize the NeoPixel LED.
    let Pio { mut common, sm0, .. } = Pio::new(p.PIO0, Irqs);
    let mut ws2812 = Ws2812::new(&mut common, sm0, p.DMA_CH0, p.PIN_15);

    // Turn off all LEDs
    let mut led1 = Output::new(p.PIN_6, Level::Low); // BLUE	<1s
    let mut led2 = Output::new(p.PIN_7, Level::Low); // GREEN	>1s
    let mut led3 = Output::new(p.PIN_8, Level::Low); // ORANGE	<5s
    let mut led4 = Output::new(p.PIN_9, Level::Low); // RED	>5s
    ws2812.write(&[(0,0,0).into()]).await;
    Timer::after_secs(2).await;

    info!("Debounce Demo");
    loop {
	ws2812.write(&[(0,0,255).into()]).await;

        // button pressed
        btn.debounce().await;
        let start = Instant::now();
        info!("Button Press");

        match with_deadline(start + Duration::from_secs(1), btn.debounce()).await {
            // Button Released < 1s
            Ok(_) => {
                info!("Button pressed for: {}ms", start.elapsed().as_millis());

		// BLUE
		led1.set_high();
		Timer::after_secs(1).await;
		led1.set_low();
                continue;
            }
            // button held for > 1s
            Err(_) => {
                info!("Button Held");

		// GREEN
		led2.set_high();
		Timer::after_secs(1).await;
		led2.set_low();
            }
        }

        match with_deadline(start + Duration::from_secs(5), btn.debounce()).await {
            // Button released <5s
            Ok(_) => {
                info!("Button pressed for: {}ms", start.elapsed().as_millis());

		// ORANGE
		led3.set_high();
		Timer::after_secs(1).await;
		led3.set_low();
                continue;
            }
            // button held for > >5s
            Err(_) => {
                info!("Button Long Held");

		// RED
		led4.set_high();
		Timer::after_secs(1).await;
		led4.set_low();
            }
        }

        // wait for button release before handling another press
        btn.debounce().await;
        info!("Button pressed for: {}ms", start.elapsed().as_millis());

	ws2812.write(&[(0,0,0).into()]).await;
	Timer::after_secs(1).await;
    }
}
