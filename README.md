# Nix-Darwin Configuration Flake

![screenshot](https://github.com/mich-murphy/nix-config/blob/master/screenshot/screenshot.png)
![screenshot-browser](https://github.com/mich-murphy/nix-config/blob/master/screenshot/screenshot-2.png)

## Contents

<!-- vim-markdown-toc Marked -->

* [Overview](#overview)
* [Installation](#installation)
* [Understanding Nix](#understanding-nix)
    * [Video Guides](#video-guides)
    * [Documentation](#documentation)
    * [References](#references)
* [Useful Repositories](#useful-repositories)

<!-- vim-markdown-toc -->

## Overview

My MacOS system configuration was created using the [Nix/NixOS](https://nixos.org/) philosophy and tooling. The end goal is to have a declarative, reliable and reproducible system configuration.

There are a number of tools used in producing the final configuration, namely:
- [Nix Flake](https://nixos.wiki/wiki/Flakes): allows for specifying of dependencies and locking of versions in configuration
- [Nix-Darwin](https://github.com/LnL7/nix-darwin): configuration of MacOS system settings
- [Home Manager](https://github.com/nix-community/home-manager): configuration of user and application settings

Once you have an understanding of Nix/NixOS, the above tools can be configured using the following references:
- [Nix Darwin - Options](https://daiderd.com/nix-darwin/manual/index.html#sec-options)
- [Home Manager - Options](https://nix-community.github.io/home-manager/options.html)
- [Home Manager - Darwin Options](https://nix-community.github.io/home-manager/nix-darwin-options.html)

## Installation

1. Install Nix on the target machine:

```bash
sh <(curl -L https://nixos.org/nix/install)
```
2. To enable installation from a Flake we need to enable experimental features for Nix

```bash
mkdir ~/.config/nix
echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf
```
3. Install Git, clone the repository and run the first build of the Flake to make darwin commands available

```bash
nix-env -iA nixpkgs.git
git clone https://github.com/mich-murphy/nix-config ~/.config/nix-config
cd ~/.config/nix-config
nix build .#darwinConfigurations.macbook.system
./result/sw/bin/darwin-rebuild switch --flake .#macbook
```
4. In future you can rebuild and activate the Flake using the following command

```bash
darwin-rebuild switch --flake .
```

## Understanding Nix

There are several components that are referred to regarding Nix:
1. Nix is the name given to a functional programming language - this is the language used for configuration
2. Nix is a package manager similar to those in other operating systems. It allows for installation of applications and dependencies
3. NixOS is an operating system, which is entirely configured using the Nix language.

The above configuration is for a Macbook Air M2 running MacOS, which does not currently have a stable version of NixOS available. All configuration is created by using the Nix package manager, with some additional options provided by Nix-Darwin.

For further information regarding Nix refer to the below resources:

### Video Guides

- [Overview & Configuration](https://github.com/MatthiasBenaets/nixos-config/blob/master/nixos.org)
- [Detailed Configuration Guide](https://www.youtube.com/watch?v=QKoQ1gKJY5A&list=PL-saUBvIJzOkjAw_vOac75v-x6EzNzZq-)
- [Technical Concepts & Guide](https://www.youtube.com/watch?v=NYyImy-lqaA&list=PLRGI9KQ3_HP_OFRG6R-p4iFgMSK1t5BHs)
- [Technical Guide Covering Packaging etc.](https://www.youtube.com/user/elitespartan117j27/videos)

### Documentation

- [Nixpkgs](https://nixos.org/manual/nixpkgs/stable)
- [NixOS](https://nixos.org/manual/nixos/stable)
- [Nix Package Manager](https://nixos.org/manual/nix/stable/command-ref/command-ref.html)
- [Nix Language](https://nixos.org/manual/nix/stable/expressions/writing-nix-expressions.html), [Learn Nix in Y Minutes](https://learnxinyminutes.com/docs/nix/)

### References 

- [Overlays - Emacs Example](https://www.heinrichhartmann.com/posts/2021-08-08-nix-into/)
- [Nix.dev - Further Material by Nix Documentation Team](https://nix.dev/)
- [Tweag.io - Blog With Useful Nix Articles](https://www.tweag.io/blog)
- [Nixos Planet - Another Blog Covering Nix](https://planet.nixos.org/)

## Useful Repositories

- [Matthias Benaet's Dotfiles](https://github.com/MatthiasBenaets/nixos-config)
    - General Nix Darwin/Home Manager/Nix Flakes overview
- [Calum MacRae's Dotfiles](https://github.com/cmacrae/config)
    - Useful reference for Firefox config
- [Dustin Lyon's Dotfiles](https://github.com/dustinlyons/nixos-config)
    - Useful reference for combining Nix Darwin and NixOS flakes
- [Jordan Isaac's Doftiles](https://github.com/jordanisaacs/dotfiles)
    - More complex principles around impermanence and building modules
- [Xe Iaso's Dotfiles](https://tulpa.dev/cadey/nixos-configs)
    - A lot of more technical concepts e.g. secrets management
