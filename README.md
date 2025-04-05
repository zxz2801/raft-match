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

## Benchmark

```test_data/benchmark.sh```

### Test Environment
- **Platform**: Mac M1
- **Concurrent Clients**: 100
- **Target Interval**: 1

### Performance Results
| Metric | Value |
|--------|-------|
| Total Requests | 822,267 |
| Average TPS | 27,408.90 |

### Latency Distribution (μs)
| Percentile | Latency |
|------------|---------|
| p50 | 1,703 |
| p90 | 2,393 |
| p95 | 2,773 |
| p99 | 5,187 |
| p99.9 | 7,815 |

## Dependencies

- `rust_decimal`: Decimal number handling
- `serde`: Serialization/deserialization
- `raft-rs`: Raft protocol 

## License

MIT License 