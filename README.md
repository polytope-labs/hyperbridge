# tesseract

<img src="assets/tesseract.jpg" />

Tesseract is a multi-~~dimensional~~chain relayer implementation for the ISMP protocol. Currently this supports relaying between:

- [x] Parachains
- [ ] Substrate based chains
- [ ] EVM based chains

## Usage Guides

Consult the [ISMP book](https://substrate-ismp.polytope.technology) for more information on how to use this in combination with the [substrate-ismp](https://github.com/polytope-labs/substrate-ismp) stack

## Docker Guide

First build the image locally:

```bash
DOCKER_BUILDKIT=0 docker build -t tesseract .
```

Next run the relayer given a config file

```bash
 docker run tesseract --config ./integration-tests/config.toml
```

## License

This software is licensed under the Apache 2.0 License, Copyright (c) 2023 Polytope Labs.