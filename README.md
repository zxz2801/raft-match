# Raft Match Engine

A high-performance matching engine implementation in Rust, designed for cryptocurrency exchanges.

## Features

- **Order Types**
  - Market Orders
  - Limit Orders
  - Support for both buy and sell sides

- **Price and Quantity Precision**
  - Configurable precision for both price and quantity
  - Support for negative precision (rounding to significant digits)
  - Automatic rounding to 1000 multiples for prices

- **Order Management**
  - Order status tracking (New, Partially Filled, Filled, Canceled, Rejected)
  - Automatic order status updates
  - Order cancellation support

- **Symbol Management**
  - Dynamic symbol creation and management
  - Configurable price and quantity limits
  - Symbol status control (Active, Inactive, Delisted)

## Architecture

The engine is organized into several key components:

- **Entry**: Core data structures
  - `Order`: Order representation and management
  - `Symbol`: Trading pair configuration
  - `Trade`: Trade execution records

- **Data**: In-memory storage
  - `OrderBook`: Order book implementation with price-time priority

- **MatchLogic**: Matching engine core
  - `Matcher`: Order matching and execution logic

- **Spot**: Trading pair management
  - `SymbolManager`: Symbol lifecycle management

- **Processor**: External interface
  - `OrderProcessor`: Order processing and validation

## Usage

```rust
use match_engine::engine::processor::OrderProcessor;
use match_engine::engine::entry::{Symbol, OrderType, OrderSide};
use rust_decimal_macros::dec;

// Create a new order processor
let mut processor = OrderProcessor::new();

// Add a new trading pair
let symbol = Symbol::new(
    "BTC/USDT".to_string(),
    "BTC".to_string(),
    "USDT".to_string(),
    2,  // price precision (2 decimal places)
    8,  // quantity precision (8 decimal places)
    dec!(10000),  // min price
    dec!(100000), // max price
    dec!(0.0001), // min quantity
    dec!(100),    // max quantity
);
processor.add_symbol(symbol)?;

// Place a limit order
let trades = processor.place_order(
    "BTC/USDT",
    OrderSide::Buy,
    OrderType::Limit,
    dec!(50000),
    dec!(0.1),
)?;
```

## Dependencies

- `rust_decimal`: Decimal number handling
- `serde`: Serialization/deserialization
- `uuid`: Unique identifier generation

## License

MIT License 