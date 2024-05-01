//! This example shows the ease of debouncing a button with async rust.
//! Hook up a button or switch between pin 9 and ground.

#![no_std]
#![no_main]

use defmt::info;

use embassy_executor::Spawner;
use embassy_rp::gpio::{AnyPin, Level, Input, Output, Pin, Pull};
use embassy_time::{Duration, Instant, Timer};
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::{Channel, Receiver};

use ws2312;
use debounce;

use {defmt_rtt as _, panic_probe as _};

enum LedStatus { On, Off }

static CHANNEL_P: Channel<ThreadModeRawMutex, LedStatus, 64> = Channel::new();
static CHANNEL_N: Channel<ThreadModeRawMutex, LedStatus, 64> = Channel::new();
static CHANNEL_R: Channel<ThreadModeRawMutex, LedStatus, 64> = Channel::new();
static CHANNEL_D: Channel<ThreadModeRawMutex, LedStatus, 64> = Channel::new();

#[derive(Copy, Clone)]
#[repr(u8)]
enum Button { P, N, R, D }

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

// ================================================================================

#[embassy_executor::task(pool_size = 4)]
async fn set_led(receiver: Receiver<'static, ThreadModeRawMutex, LedStatus, 64>, led_pin: AnyPin) {
    let mut led = Output::new(led_pin, Level::Low);

    loop {
	match receiver.try_receive() {
	    Ok(LedStatus::On)  => led.set_high(),
	    Ok(LedStatus::Off) => led.set_low(),
	    _ => Timer::after_millis(250).await, // Don't allow another button for quarter second.
	}
    }
}

#[embassy_executor::task(pool_size = 4)]
async fn read_button(
    spawner: Spawner,
    button:  Button,
    btn_pin: AnyPin,
    led_pin: AnyPin)
{
    let mut btn = debounce::Debouncer::new(Input::new(btn_pin, Pull::Up), Duration::from_millis(20));

    // Spawn off a LED driver for this button.
    let receiver: Receiver<'static, ThreadModeRawMutex, LedStatus, 64>;
    match button {
	Button::P  => receiver = CHANNEL_P.receiver(),
	Button::N  => receiver = CHANNEL_N.receiver(),
	Button::R  => receiver = CHANNEL_R.receiver(),
	Button::D  => receiver = CHANNEL_D.receiver(),
    }
    spawner.spawn(set_led(receiver, led_pin)).unwrap();

    loop {
        // button pressed
        btn.debounce().await;
        let start = Instant::now();
        info!("Button Press");

	// Don't really care how long a button have been pressed as,
	// the `debounce()` will detect when it's been RELEASED.
        match btn.debounce().await {
            _ => {
                info!("Button pressed for: {}ms", start.elapsed().as_millis());

		// We know who WE are, so turn ON our own LED and turn off all the other LEDs.
		// Turn on our OWN LED.
		match button {
		    Button::P  => {
			CHANNEL_P.send(LedStatus::On).await;
			CHANNEL_N.send(LedStatus::Off).await;
			CHANNEL_R.send(LedStatus::Off).await;
			CHANNEL_D.send(LedStatus::Off).await;
		    }
		    Button::N  => {
			CHANNEL_P.send(LedStatus::Off).await;
			CHANNEL_N.send(LedStatus::On).await;
			CHANNEL_R.send(LedStatus::Off).await;
			CHANNEL_D.send(LedStatus::Off).await;
		    }
		    Button::R  => {
			CHANNEL_P.send(LedStatus::Off).await;
			CHANNEL_N.send(LedStatus::Off).await;
			CHANNEL_R.send(LedStatus::On).await;
			CHANNEL_D.send(LedStatus::Off).await;
		    }
		    Button::D  => {
			CHANNEL_P.send(LedStatus::Off).await;
			CHANNEL_N.send(LedStatus::Off).await;
			CHANNEL_R.send(LedStatus::Off).await;
			CHANNEL_D.send(LedStatus::On).await;
		    }
		}

		// wait for button release before handling another press
		btn.debounce().await;
		info!("Button pressed for: {}ms", start.elapsed().as_millis());
            }
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Start");

    let p = embassy_rp::init(Default::default());

    // =====
    // Initialize the NeoPixel LED.
    let Pio { mut common, sm0, .. } = Pio::new(p.PIO0, Irqs);
    let mut ws2812 = ws2312::Ws2812::new(&mut common, sm0, p.DMA_CH0, p.PIN_15);

    // Spawn off one button reader per button.
    spawner.spawn(read_button(spawner, Button::P, p.PIN_2.degrade(), p.PIN_6.degrade())).unwrap(); // button/P
    spawner.spawn(read_button(spawner, Button::N, p.PIN_3.degrade(), p.PIN_7.degrade())).unwrap(); // button/N
    spawner.spawn(read_button(spawner, Button::R, p.PIN_4.degrade(), p.PIN_8.degrade())).unwrap(); // button/R
    spawner.spawn(read_button(spawner, Button::D, p.PIN_5.degrade(), p.PIN_9.degrade())).unwrap(); // button/D

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
