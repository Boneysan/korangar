<img align="left" alt="" src=".github/logo.png" height="130" />

# [Korangar](https://github.com/vE5li/korangar)

[![Build](https://github.com/ve5li/korangar/actions/workflows/build.yml/badge.svg?branch=main)](https://github.com/ve5li/korangar/actions?query=workflow%3ABuild)
[![Tests](https://github.com/ve5li/korangar/actions/workflows/tests.yml/badge.svg?branch=main)](https://github.com/ve5li/korangar/actions?query=workflow%3ATests)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](https://opensource.org/licenses/MIT)
[![Discord](https://img.shields.io/discord/1010572689536204931?label=discord)](https://discord.gg/2CqRZsvKja)

Korangar is a next-gen Ragnarok Online client written in Rust. It features real-time lighting with drop shadows, a completely new user interface, and removes limitations of the official client.

> [!IMPORTANT]
> **This specific fork** is a specialized version of Korangar being built as a **Native Tabletop Tooling** engine for a custom D&D-style campaign ("Seal Cascade"). It extends the base engine with DM-specific tools, such as animated dice cards, an initiative tracker, a campaign board, and custom UI elements for running live sessions.
> 
> **For more details, see our custom design docs:**
> - [Documentation Hub](docs/README.md) — Complete index of all technical documentation
> - [DM Interface & Native Tabletop Tooling](docs/DM_INTERFACE.md)
> - [Software Design & Architecture](docs/SOFTWARE_DESIGN.md)
> - [Feature Roadmap](docs/FEATURE_ROADMAP.md)
> - [Project Plan](docs/PROJECT_PLAN.md)

##### Screenshot of the current state
![geffen](.github/geffen.png)

## 🚀 Running

> [!IMPORTANT]
> Korangar is still very early in development and is anything but feature-complete.

If you want to try it out for yourself, check out the [Installation page](wiki/Installation.md).

## 🔧 Troubleshooting

If you're running into issues while setting up or running Korangar please check the [Troubleshooting page](wiki/Troubleshooting.md). In case your issue is not listed feel free to [create an issue](https://github.com/vE5li/korangar/issues/new) or use the dedicated `support` channel on our [Discord server](https://discord.gg/2CqRZsvKja).

## 🤝 Contributing

This is a very ambitious project and we are always looking for contributors. If you are interested, please read [this page](wiki/Contributing.md).

## 🔥 Updates

There is a dedicated channel for `updates` on our [Discord server](https://discord.gg/2CqRZsvKja). If you want to stay up to date with development or see recent changes, go check it out!

## 📦 Packages

We try to keep the project as modular as possible by splitting the codebase into individual crates. All the crates prefixed with `ragnarok-` are **independent of Korangar and have no dependencies on it**.

We encourage everyone to use these crates for their own Ragnarok Online related projects and contribute back if they want to.
