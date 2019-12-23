#![no_main]
#![no_std]

extern crate panic_semihosting;

use stm32f4xx_hal as hal;
use ws2811_spi as ws2811;

use hal::spi::*;
use hal::{prelude::*, stm32};

use ws2811::Ws2811;

use smart_leds::SmartLedsWrite;
use smart_leds_trait::RGB8;

// use cortex_m_semihosting::hprintln;

use rtfm::cyccnt::U32Ext;

const PERIOD: u32 = 48_000_000 / 2;
const MAX_LEDS: usize = 50;

// Types for WS
use hal::gpio::gpiob::{PB3, PB5};
use hal::gpio::{Alternate, AF5};
use hal::spi::{NoMiso, Spi};
use hal::stm32::SPI1;

type Pins = (PB3<Alternate<AF5>>, NoMiso, PB5<Alternate<AF5>>);

#[rtfm::app(device = stm32f4xx_hal::stm32, peripherals = true, monotonic = rtfm::cyccnt::CYCCNT)]
const APP: () = {
    struct Resources {
        ws: Ws2811<Spi<SPI1, Pins>>,
        data: [RGB8; MAX_LEDS],
    }

    #[init(schedule = [walk])]
    fn init(mut cx: init::Context) -> init::LateResources {
        // Device specific peripherals
        let dp: stm32::Peripherals = cx.device;

        // Set up the system clock at 48MHz
        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(48.mhz()).freeze();

        // Initialize (enable) the monotonic timer (CYCCNT)
        cx.core.DCB.enable_trace();
        cx.core.DWT.enable_cycle_counter();

        // Configure pins for SPI
        // We don't connect sck, but I think the SPI traits require it?
        let gpiob = dp.GPIOB.split();
        let sck = gpiob.pb3.into_alternate_af5();

        // Master Out Slave In - pb5, Nucleo 64 pin d4
        let mosi = gpiob.pb5.into_alternate_af5();

        let spi = Spi::spi1(
            dp.SPI1,
            (sck, NoMiso, mosi),
            Mode {
                polarity: Polarity::IdleLow,
                phase: Phase::CaptureOnFirstTransition,
            },
            stm32f4xx_hal::time::KiloHertz(3000).into(),
            clocks,
        );

        let mut ws = Ws2811::new(spi);

        let brightness = 128;
        let red = RGB8 { r: brightness, g: 0, b: 0 };
        let green = RGB8 { r: 0, g: brightness, b: 0 };
        let blue = RGB8 { r: 0, g: 0, b: brightness };

        let pattern = [red, green, blue];

        let mut data = [RGB8::default(); MAX_LEDS];
        for (i, colour) in pattern.iter().cycle().take(MAX_LEDS).enumerate() {
            data[i] = *colour;
        }

        ws.write(data.iter().cloned())
            .expect("Failed to write lights_on");

        cx.schedule
            .walk(cx.start + PERIOD.cycles())
            .expect("failed schedule initial walk");

        init::LateResources { ws, data }
    }

    #[task(schedule = [walk], resources = [ws, data])]
    fn walk(cx: walk::Context) {
        let walk::Resources { ws, data } = cx.resources;

        let decr = |v| if v == 0 { 0 } else { v - 1 };

        for i in 0..MAX_LEDS {
            let colour = RGB8 {
                r: decr(data[i].r),
                g: decr(data[i].g),
                b: decr(data[i].b),
            };

            data[i] = colour;
        }

        ws.write(data.iter().cloned()).expect("write in walk");

        cx.schedule
            .walk(cx.scheduled + PERIOD.cycles())
            .expect("Failed to schedule walk");
    }

    extern "C" {
        fn USART1();
    }
};
