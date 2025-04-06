use clap::Parser;
use hdrhistogram::Histogram;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;

use pb::match_service_client::MatchServiceClient;
use pb::{
    CreateSymbolRequest, Order, OrderSide, OrderType, PlaceOrderRequest, Symbol, SymbolStatus,
    TimeInForce,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Number of concurrent clients
    #[arg(short, long, default_value = "1")]
    concurrency: usize,

    /// INTERVAL ms
    #[arg(short, long, default_value = "100")]
    interval: u64,

    /// Duration of the benchmark in seconds
    #[arg(short, long, default_value = "30")]
    duration: u64,

    /// Server address
    #[arg(short, long, default_value = "grpc://127.0.0.1:4001")]
    server: String,
}

#[allow(clippy::module_inception)]
pub mod pb {
    tonic::include_proto!("r#match");
}

async fn create_symbol(server_addr: &str) {
    let mut client = match MatchServiceClient::connect(server_addr.to_string()).await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Failed to connect to server: {}", e);
            return;
        }
    };
    let request = tonic::Request::new(CreateSymbolRequest {
        symbol: Some(Symbol {
            symbol: "BTCUSDT".to_string(),
            base: "BTC".to_string(),
            quote: "USDT".to_string(),
            min_quantity: "0.000001".to_string(),
            max_quantity: "1000000".to_string(),
            min_amount: "0.000001".to_string(),
            max_amount: "1000000".to_string(),
            price_precision: 5,
            quantity_precision: 5,
            status: SymbolStatus::Alive as i32,
        }),
    });
    match client.create_symbol(request).await {
        Ok(_) => println!("Symbol created"),
        Err(e) => eprintln!("Failed to create symbol: {}", e),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Connect to the server
    let server_addr = args.server.clone();
    let histogram = Arc::new(Mutex::new(Histogram::<u64>::new(3).unwrap()));
    let total_requests = Arc::new(Mutex::new(0u64));

    println!(
        "Starting benchmark with {} concurrent clients, target INTERVAL: {}",
        args.concurrency, args.interval
    );

    create_symbol(&server_addr).await;

    // Spawn client tasks
    let mut handles = vec![];
    for _ in 0..args.concurrency {
        let server_addr = server_addr.clone();
        let histogram = histogram.clone();
        let total_requests = total_requests.clone();

        let handle = tokio::spawn(async move {
            let mut client = match MatchServiceClient::connect(server_addr).await {
                Ok(client) => client,
                Err(e) => {
                    eprintln!("Failed to connect to server: {}", e);
                    return;
                }
            };

            loop {
                let start = Instant::now();

                // Send request
                let request = tonic::Request::new(PlaceOrderRequest {
                    order: Some(Order {
                        symbol: "BTCUSDT".to_string(),
                        account_id: rand::random::<u64>(),
                        order_side: OrderSide::Buy as i32,
                        order_type: OrderType::Limit as i32,
                        time_in_force: TimeInForce::Gtc as i32,
                        quantity: "0.001".to_string(),
                        price: "50000.0".to_string(),
                        order_id: rand::random::<u64>() % 1000,
                        taker_fee: "0.0005".to_string(),
                        maker_fee: "0.0005".to_string(),
                    }),
                });

                // println!("Request sent {:?}", request);
                match client.place_order(request).await {
                    Ok(_) => {
                        let duration = start.elapsed();
                        let mut hist = histogram.lock().await;
                        hist.record(duration.as_micros() as u64).unwrap();
                        let mut total = total_requests.lock().await;
                        *total += 1;
                        // println!("Request success cost: {} us", duration.as_micros());
                    }
                    Err(e) => eprintln!("Request failed: {}", e),
                }

                tokio::time::sleep(Duration::from_millis(args.interval)).await;
            }
        });

        handles.push(handle);
    }

    // Run for specified duration
    sleep(Duration::from_secs(args.duration)).await;

    // Cancel all tasks
    for handle in handles {
        handle.abort();
    }

    // Print statistics
    let total = *total_requests.lock().await;
    let hist = histogram.lock().await;

    println!("\nBenchmark Results:");
    println!("Total Requests: {}", total);
    println!("Average TPS: {:.2}", total as f64 / args.duration as f64);
    println!("\nLatency Distribution (microseconds):");
    println!("p50: {}", hist.value_at_percentile(50.0));
    println!("p90: {}", hist.value_at_percentile(90.0));
    println!("p95: {}", hist.value_at_percentile(95.0));
    println!("p99: {}", hist.value_at_percentile(99.0));
    println!("p99.9: {}", hist.value_at_percentile(99.9));

    Ok(())
}
