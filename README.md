# Layered System

> A "Time Machine" for Windows based on VHDX Differencing Chains.
>
> 基于 VHDX 差分链的 Windows 分层系统管理工具。

## Introduction

Layered System 允许你像管理 Git 分支一样管理你的 Windows 系统。它利用 Windows 原生的 VHDX 差分技术，让你能够在秒级时间内创建、切换、回滚系统环境。

不再因为安装了流氓软件而重装系统，不再因为开发环境冲突而头秃。你可以在一个干净的“母盘”上，为不同的项目、游戏或测试需求创建独立的“子盘”。

## Features

- **Git-like System Management**: Manage your system like git branches (Base -> Feature A / Feature B).
- **Native Performance**: Runs on bare metal, no virtualization overhead (unlike VMware/VirtualBox).
- **Instant Switch**: Switch between different system states in seconds.
- **Space Efficient**: Only stores differences in child nodes.
- **Portable**: VHDX files are stored in a single directory (except for BCD entries).

## How it Works

Layered System leverages native Windows commands:

- **Diskpart**: To create and manage VHDX files.
- **DISM**: To apply Windows images (WIM/ESD) to VHDX.
- **Bcdboot**: To make VHDX bootable.
- **Bcdedit**: To manage boot entries.

## Tech Stack

- **Frontend**: React + TypeScript + Tailwind CSS
- **Backend**: Rust (Tauri Framework)
- **Database**: SQLite

## Prerequisites

- Windows 10 or Windows 11
- Administrator privileges (Required for disk and BCD operations)
- [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (Usually installed by default on modern Windows)

## Getting Started

1. Clone the repository:

   ```bash
   git clone https://github.com/hamflx/layered-system.git
   cd layered-system
   ```

2. Install dependencies:

   ```bash
   bun install
   ```

3. Run in development mode:

   ```bash
   bun run tauri dev
   ```

## Disclaimer / 免责声明

**WARNING**: This tool modifies your system's Boot Configuration Data (BCD). While we have implemented safety checks, there is always a risk when manipulating system boot records.
**警告**：本工具涉及修改系统引导记录 (BCD)。虽然我们做了很多安全检查，但在尝试之前，**强烈建议备份您的重要数据**。

## License

[MIT](LICENSE)
