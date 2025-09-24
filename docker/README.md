# SP BCE Settlement System - Docker Setup

This directory contains the containerized SP BCE settlement system built on real Albatross blockchain infrastructure.

## 🏗️ Architecture

- **3 SP BCE Validators**: Real consensus using Albatross
- **MDBX Storage**: Persistent blockchain data
- **P2P Network**: Validator communication
- **Settlement Logic**: Threshold-based BCE record processing (>100 EUR → blockchain)

## 🚀 Quick Start

```bash
# Build and start the settlement system
cd /home/zeljko/src/albatross-fresh/sp-bce-node/docker
docker compose up --build -d

# Check validator health
curl http://localhost:8081/health
curl http://localhost:8082/health
curl http://localhost:8083/health

# Run settlement tests
docker compose exec sp-bce-test-client ./run_tests.sh
```

## 🧪 Testing

The system includes comprehensive tests:

- **Small Settlements**: Records < 100 EUR (stay in memory)
- **Large Settlements**: Records > 100 EUR (trigger blockchain writes)
- **Realistic Scenario**: 95% small, 5% large settlements
- **Multi-Validator**: Test consensus across validators

## 📊 API Endpoints

Each validator exposes:
- `GET /health` - Health check
- `POST /api/v1/bce/submit` - Submit BCE record
- `GET /api/v1/bce/stats` - Settlement statistics
- `GET /api/v1/network/status` - Network status

## 🔧 Configuration

Environment variables:
- `SETTLEMENT_THRESHOLD_CENTS`: Blockchain threshold (default: 10000 = 100 EUR)
- `NODE_ID`: Validator identifier
- `API_PORT`: API server port
- `P2P_PORT`: P2P network port
- `BOOTSTRAP_PEERS`: Initial P2P peers

## 🏥 Monitoring

```bash
# View logs
docker compose logs -f sp-bce-validator-1
docker compose logs -f sp-bce-validator-2
docker compose logs -f sp-bce-validator-3

# Check statistics
curl http://localhost:8081/api/v1/bce/stats | jq
```

## 🛑 Cleanup

```bash
# Stop and remove containers
docker compose down

# Remove volumes (deletes blockchain data)
docker compose down -v
```

This provides a complete containerized SP BCE settlement system for telecom billing reconciliation using real Albatross blockchain consensus.