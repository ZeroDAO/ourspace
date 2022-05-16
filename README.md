![Logo](https://pic.tom24h.com/orsourspace-index.png)

<div align="center">

[![Substrate version](https://img.shields.io/badge/Substrate-3.0.0-brightgreen?logo=Parity%20Substrate)](https://substrate.dev/)
[![codecov](https://codecov.io/gh/ZeroDAO/ourspace/branch/main/graph/badge.svg)](https://app.codecov.io/gh/ZeroDAO/ourspace)
[![License](https://img.shields.io/github/license/ZeroDAO/ourspace?color=green)](https://github.com/ZeroDAO/ourspace/blob/main/LICENSE)
 <br />
[![Twitter URL](https://img.shields.io/twitter/url?style=social&url=https%3A%2F%2Ftwitter.com%2Fzerodaonet)](https://twitter.com/zerodaonet)
[![Discord](https://img.shields.io/badge/Discord-gray?logo=discord)](https://discord.gg/K56C6jtr)
[![Medium](https://img.shields.io/badge/Medium-gray?logo=medium)](https://zerodao.medium.com/)

</div>

**We are planning the next edition. It adds to the graph database by fetching social graph data from each chain. Not only does it run the TIR algorithm to calculate reputation, but also, anyone can deploy smart contracts to calculate the social graph.**

For technical and guides, please refer to the [ZeroDAO Docs](https://docs.zerodao.net/).

# 1. Introduction

We define Ourspace as a public resource, including a social network, a reputation system. the Ourspace social network solves the incentive dilemma that currently exists in blockchain social networks, while incentivizing good behavior makes good behavior disappear. Imagine what Twitter would look like if you could get $1 for posting a tweet. Two-factor theory even concludes that security, salary, fringe benefits, good pay is not Motivators but Hygiene factors. Hygiene factors that do not give positive satisfaction or lead to higher motivation.

Ourspace social network solves the incentive dilemma by amplifying social motivation and internalizing external motivation.

In the Ourspace network, we still quantify user contributions and settle them into Tokens, which we call social currency. It is frozen and at some point assigned to users trusted by the owner, it is also social currency and goes on to be shared. The user's social motivation is amplified. We use to shared information, now we share value.

Ourspace social network brought us the reputation system and we proposed the TIR algorithm to compute the graph and obtain the reputation of each user. TIR is difficult to compute but easy to verify on-chain. This feature makes Ourspace's reputation system completely decentralized. At the same time, it has strong ability to prevent Sybil Attack to meet the security needs of financial products and on-chain governance. Ourspace also brings credit finance, zero-cost payments, and other applications to the blockchain.

![Web3 Grants](https://github.com/ZeroDAO/www.ourspace.network/blob/main/src/assets/images/w3f.svg)

# 2. Building

## Initial Setup

### Setup rust

```bash
curl https://sh.rustup.rs -sSf | sh
rustup update stable
```

### You will also need to install the following packages:

#### Mac

```bash
brew install cmake pkg-config openssl git llvm
```

#### Linux

```bash
sudo apt install cmake pkg-config libssl-dev git clang libclang-dev
```

### More

Ourspace is based on Substrate, for more information please go to [Substrate](https://docs.substrate.io/v3/getting-started/overview/).

## Installation

```bash
make init
```

## Build

```bash
make build-release
```

# 3. Dev Network

## 1. Run as dev

```bash
make run-dev
```

## 2. Run as local

Start the local blockchain node using the `alice` account by running the following command:

```bash
./target/release/ourspace \
  --base-path /tmp/alice \
  --chain local \
  --alice \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001
  --validator
```

Start a second local blockchain node using the `bob` account by running the following command:

```bash
./target/release/ourspace \
  --base-path /tmp/bob \
  --chain local \
  --bob \
  --port 30334 \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp
  --validator
```

# 4. Development

## Test All

```bash
make test
```

## Purge the development chain

```bash
make purge-dev
```

# Docker

## Start a single chain

```bash
./scripts/docker_run.sh
```

You can also

```bash
# Run Ourspace node without re-compiling
./scripts/docker_run.sh ./target/release/ourspace --dev --ws-external

# Purge the local dev chain
./scripts/docker_run.sh ./target/release/ourspace purge-chain --dev
```

# References
- [Substrate repo](https://github.com/paritytech/substrate)
- [Substrate Developer Hub](https://substrate.dev/)
- [substrate-node-template](https://github.com/substrate-developer-hub/substrate-node-template)
- [ORML](https://github.com/open-web3-stack/open-runtime-module-library)
