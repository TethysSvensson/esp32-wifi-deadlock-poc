#![no_std]
#![no_main]

use core::future;

use embassy_executor::Spawner;
use embassy_net::{tcp::TcpSocket, Stack};
use embassy_time::{Duration, Timer};
use embedded_io_async::Write;
use esp_backtrace as _;
use esp_hal::{clock::CpuClock, rng::Rng, timer::timg::TimerGroup};
use esp_println::println;

mod wifi;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(72 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let timg1 = TimerGroup::new(peripherals.TIMG1);
    let rng = Rng::new(peripherals.RNG);

    esp_hal_embassy::init(timg1.timer0);

    let stack = wifi::init_wifi(
        &spawner,
        timg0.timer0,
        rng,
        peripherals.RADIO_CLK,
        peripherals.WIFI,
    );

    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    println!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            println!("Got IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    for _ in 0..wifi::MAX_CONNECTIONS {
        spawner.spawn(echo_server(stack, 1337)).unwrap();
    }

    future::pending().await
}

#[embassy_executor::task(pool_size = wifi::MAX_CONNECTIONS)]
async fn echo_server(stack: Stack<'static>, port: u16) {
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut tcp_buf = [0; 4096];
    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    'accept_loop: loop {
        println!("listening");
        Timer::after(Duration::from_millis(500)).await;
        if let Err(e) = socket
            .accept(embassy_net::IpListenEndpoint { addr: None, port })
            .await
        {
            println!("Error accepting: {e:?}");
            socket.close();
            Timer::after(Duration::from_millis(1000)).await;
        }
        socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));
        loop {
            let n = match socket.read(&mut tcp_buf[..]).await {
                Ok(n) => n,
                Err(e) => {
                    println!("Error receiving: {e:?}");
                    socket.close();
                    continue 'accept_loop;
                }
            };
            if n == 0 {
                socket.close();
                continue 'accept_loop;
            }
            match embassy_time::with_timeout(
                embassy_time::Duration::from_secs(10),
                socket.write_all(&tcp_buf[..n]),
            )
            .await
            {
                Err(e) => {
                    println!("Timeout while writing: {e:?}");
                    socket.close();
                    continue 'accept_loop;
                }
                Ok(Err(e)) => {
                    println!("Error while writing: {e:?}");
                    socket.close();
                    continue 'accept_loop;
                }
                Ok(Ok(())) => (),
            }
        }
    }
}
