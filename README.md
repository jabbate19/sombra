# RShell - A C2 in Rust using ICMP Ping

This repository contains the client, server, persistence, and ansible for this program.

## ICMP Protocol

This C2 takes advantage of ICMP Ping packets to send and receive responses. The server sends a ping to the client with a keyword and command at the end of the packet. The client picks up on this keyword, executes the command, and responds with the command output and a different keyword. The data simply looks like pings, when in reality it is C2 traffic.

## Client

The client checks if it is already running via a specified file holding a PID. It checks if this PID is running the same executable as itself, stopping if it matches. Any command sent with the keyword is processed and ran, having the response sent back to the server.

## Server

The server uses a Cursive TUI that allows the user to view systems in different orderings. VMs can be filtered by hostname, OS, or Team # and have commands executed on multiple systems at once.

## Persistence

The persistence used for this is hooking onto common commands or executables. On Linux/FreeBSD, the ls command is hooked. On Windows, cmd.exe is hooked. The C programs for this are stored in this repository.

## Ansible

Ansible is readily available to deploy on Windows, Linux, and FreeBSD systems. The only change that needs to be made is the hostnames.