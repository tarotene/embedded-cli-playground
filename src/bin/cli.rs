#![no_main]
#![no_std]

use defmt_rtt as _;
use panic_probe as _;

use playground as _; // global logger + panicking-behavior + memory layout

use stm32f3xx_hal::prelude::*;

#[allow(dead_code)]
struct Writer {
    serial: stm32f3xx_hal::serial::Serial<
        stm32f3xx_hal::pac::UART4,
        (
            stm32f3xx_hal::gpio::PC10<
                stm32f3xx_hal::gpio::Alternate<stm32f3xx_hal::gpio::OpenDrain, 5>,
            >,
            stm32f3xx_hal::gpio::PC11<
                stm32f3xx_hal::gpio::Alternate<stm32f3xx_hal::gpio::OpenDrain, 5>,
            >,
        ),
    >,
}

impl embedded_io::ErrorType for Writer {
    type Error = embedded_io::ErrorKind;
}

impl embedded_io::Write for Writer {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        for &b in buf {
            stm32f3xx_hal::nb::block!(self.serial.write(b)).unwrap();
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        // TODO: エラー情報ぶち抜く
        stm32f3xx_hal::nb::block!(self.serial.flush()).unwrap();
        Ok(())
    }
}

#[cortex_m_rt::entry]
fn main() -> ! {
    let dp = stm32f3xx_hal::pac::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.freeze(&mut dp.FLASH.constrain().acr);

    let mut gpioc = dp.GPIOC.split(&mut rcc.ahb);

    let serial = stm32f3xx_hal::serial::Serial::new(
        dp.UART4,
        (
            gpioc.pc10.into_af_open_drain::<5>(
                &mut gpioc.moder,
                &mut gpioc.otyper,
                &mut gpioc.afrh,
            ),
            gpioc.pc11.into_af_open_drain::<5>(
                &mut gpioc.moder,
                &mut gpioc.otyper,
                &mut gpioc.afrh,
            ),
        ),
        115200.Bd(),
        clocks,
        &mut rcc.apb1,
    );

    let writer = Writer { serial: serial };

    let (command_buffer, history_buffer) = unsafe {
        static mut COMMAND_BUFFER: [u8; 16] = [0; 16];
        static mut HISTORY_BUFFER: [u8; 16] = [0; 16];
        (COMMAND_BUFFER.as_mut(), HISTORY_BUFFER.as_mut())
    };

    defmt::info!("Creating CLI...");

    let mut cli = embedded_cli::cli::CliBuilder::default()
        .writer(writer)
        .command_buffer(command_buffer)
        .history_buffer(history_buffer)
        .build()
        .ok()
        .unwrap();

    defmt::info!("CLI created!");

    let _ = cli.write(|writer| {
        writer.write_str("Welcome to the embedded CLI!\r\n")?;
        Ok(())
    });

    defmt::info!("Entering loop...");

    loop {}
}
