+++
title = "Operating OpenStack without fear"
date = 2026-10-26
description = "A calm upgrade starts with a map of the control plane, a tested rollback, and a clear definition of done."
template = "blog/page.html"

[taxonomies]
tags = ["openstack", "operations", "linux"]

[extra]
demo = true
+++
OpenStack upgrades have a reputation for drama. Most of the drama I have seen did not come from the upgrade itself; it came from the assumptions around it — undocumented local changes, untested database migrations, and a rollback plan that existed only in someone’s head.

## Start with a map

Before touching a control-plane node, draw the system as it actually is:

- Which services run on which hosts?
- Which database and message queue do they depend on?
- Where does the API traffic enter, and what checks the health of each endpoint?
- Which tenant networks cross which physical interfaces?
- What is the storage path for a volume that is attached to a running instance?

The drawing does not need to be beautiful. It needs to be true. A good diagram makes the upgrade order obvious, and it exposes the components that have no obvious owner.

## Define done

An upgrade is not done when the packages install. It is done when:

1. Every API endpoint reports the expected version.
2. A new instance can boot, attach a volume, and reach the network.
3. An existing instance survives a live migration.
4. The control-plane services survive a restart of one node.
5. The monitoring is quiet for the right reasons, not because it stopped collecting.

## Keep a rollback path

A rollback is not a time machine. Database migrations, message queue schemas, and agent versions can make a clean rollback impossible. Decide in advance which parts are reversible, which parts are forward-only, and which parts need a maintenance window.

## The habit

The best upgrade is the one that feels boring. That does not happen by accident. It happens when the system is mapped, the success criteria are written down, and the team has practiced the failure modes before they are real.

That is the work I care about: not making infrastructure look complicated, but making it legible enough to operate calmly.
