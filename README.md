# fitbit-grabber
This is a small utility for fetching data from the FitBit web API. It currently supports:

- Heart Rate

## Building

```sh
cargo build [--release]
```

## Using

You will need to have an OAuth 2.0 token to use `fitbit-grabber`. The reason for this requirement is that intraday heart rate time series data is only available for persoanl applications.

**TODO** Document setting up an application

```sh
./target/[debgug|release]/fitbit-grabber <starting date range>
```

For example

```sh
./target/release/fitbit-grabber 2017-10-01
```

If everything works ok, you should have one JSON file for every date from the given starting date to today.
