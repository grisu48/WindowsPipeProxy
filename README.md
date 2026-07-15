# Windows Pipe Proxy

The Windows Pipe Proxy or WPP is a named pipe to tcp proxy. It acts as a bridge between a set of [Windows named pipes](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipes) and TCP sockets.
Each named pipe will be exposed to a defined TCP socket. The tool can be used for connecting a serial port on a Hyper-V hypervisor to an [openQA](https://open.qa/) instance.

Additional documentation is provided in [docs/](docs/).

## Usage

WPP acts as a drop-in replacement for the aging `Named Pipe TCP Proxy`. It is configured via a [configuration file](pipe-proxy.toml) in `C:\pipe-proxy.toml`. The program supports a set of named pipes and will run just in the background. Once a named pipe appears, it will open a single TCP socket on a defined local address for this pipe.

To connect to a Hyper-V instance, one needs to first add a COM to named pipe option to the virtual machine, e.g. for a VM named `jellyfish` one could run:

```
Set-VMComPort -VMName jellyfish -Number 1 -Path \\.\pipe\jellyfish
```

## Credits

* Inspired by https://github.com/pratikpc/named-pipe-to-tcp-proxy
