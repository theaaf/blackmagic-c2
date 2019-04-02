# blackmagic-c2

This is a proof-of-concept command and control mechanism for Blackmagic devices.

It consists of an "agent" (the on-premises component) and a "hub" (the cloud component) that currently just run together in a single process. The hub provides a GraphQL API and has a Typescript / React UI that allows you to monitor and control DeckLink and networked devices.

To run it:

* Run `npm install` and `npm run build` from the src/hub/ui directory to build the hub UI.
* Run `cargo run` from the repo's root.
