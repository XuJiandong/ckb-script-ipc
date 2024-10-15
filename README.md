# CKB Script IPC

This project consists of two main components: a proc-macro library for
generating Inter-Process Communication (IPC) code for CKB scripts, and a common
runtime library for CKB script IPC functionality.

## Overview

The `ckb-script-ipc` crate provides procedural macros that simplify the process
of creating IPC code for CKB scripts. It automates the generation of
serialization, deserialization, and communication boilerplate, allowing
developers to focus on the core logic of their scripts.

The `ckb-script-ipc-common` crate offers a set of tools and runtime support for
IPC in CKB scripts. It includes necessary dependencies and features to
facilitate communication between different parts of a CKB script.

## Features

- Automatic generation of IPC message structures
- Serialization and deserialization of IPC messages
- Easy-to-use macros for defining IPC interfaces
- Support for CKB script-specific IPC patterns

## Usage
