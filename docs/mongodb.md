# MongoDB Integration

Oxide supports MongoDB through `oxide-framework-mongodb`.

## Install

```toml
[dependencies]
oxide_framework_core = { path = "../oxide-framework-core" }
oxide_framework_mongodb = { path = "../oxide-framework-mongodb" }
```

## Register in App

```rust
use oxide_framework_core::App;
use oxide_framework_mongodb::{AppMongoExt, MongoConfig};

let cfg = MongoConfig::new("mongodb://localhost:27017", "mydb").strict(true);

App::new().mongodb(cfg);
```

Strict mode adds a ping-backed readiness check for `/health/ready`.

## Inject Handle

Use `Data<MongoHandle>` in handlers and access `handle.db` for collections.
