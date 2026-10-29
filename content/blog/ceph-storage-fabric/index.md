+++
title = "A Ceph storage fabric with dual-leaf redundancy"
date = 2026-10-28
description = "Three monitor hosts and three OSD hosts, each with two dual-port NICs, dual-homed to a pair of leaf switches."
template = "blog/page.html"

[taxonomies]
tags = ["ceph", "networking", "architecture"]

[extra]
demo = true
+++
A Ceph cluster is only as reliable as the fabric underneath it. The monitor quorum needs low, predictable latency, and the OSDs need enough independent paths that losing a NIC, a cable, or a leaf switch does not turn into a storage outage.

The logical view is deliberately simpler: it shows which hosts participate in the Ceph public network and which participate in the cluster network.

{{<diagram name="ceph-storage-fabric-simple" caption="Ceph logical networks: the public network connects clients, monitors, and OSDs, while the cluster network carries MON and OSD traffic." />}}

The physical view below expands that into NICs, ports, leaf switches, and individual links.

{{<diagram name="ceph-storage-fabric" caption="Three MON hosts and three OSD hosts, each with two dual-port NICs, dual-homed to two leaf switches. The bond crosses NICs and leafs." />}}

## The topology

Six physical hosts sit below a pair of 48-port leaf switches. Each host has two dual-port NICs:

- **NIC 1** has two ports.
- **NIC 2** has two ports.
- Every port is connected: one port from each NIC goes to `leaf-01`, and one port from each NIC goes to `leaf-02`.

That gives every host four independent paths to the fabric. The useful redundancy comes from the fact that the members of the bond come from different NICs and land on different leaf switches. A single failed NIC or cable cannot take the host off the network, and a single leaf failure still leaves half of the bond alive.

For a single bond to span both leaf switches, the leaves need to behave as one logical LAG endpoint — normally an MLAG or a stacked pair. Without that, use separate bonds per leaf and let multipath routing handle failover at layer 3.

## Logical networks

The public network is the client-facing path: clients reach RGW/MON services and the OSDs accept front-end traffic. The cluster network is the private back-end for monitor and OSD replication and heartbeat traffic. Keeping them separate makes it easier to reason about latency and failure isolation, especially during recovery.

A host can be on both networks. In the logical diagram, the two colored rails show that membership directly; in the physical diagram, the same host has separate NICs and ports for each path.

## Reading the port grid

Each leaf shows its full 48-port grid. Port numbers 1–12 are in use: six hosts contribute two links each. The highlighted ports are the ones carrying links in this diagram.

On a host, the two NIC blocks show the port-level wiring directly:

- `nic1/1` → `leaf-01`
- `nic1/2` → `leaf-02`
- `nic2/1` → `leaf-01`
- `nic2/2` → `leaf-02`

The blue and amber port cells distinguish the two NICs. All four links for a host share the host’s bond color, so the redundant bond reads as one unit on the fabric side; the leaf port numbers make the same mapping visible from the switch side. Hovering any host highlights its four links and dims the rest; the diagram is also pan/zoomable for detail.

## What I check first

- **MLAG health** on the leaf pair. A split MLAG is worse than a single switch.
- **Bond member state** on every host. A bond that has silently lost half its members still passes traffic, but no longer has the redundancy you designed.
- **LACP hashing and link utilization.** Four links do not help if one flow can only use one member.
- **Monitor network latency.** Keep the MON quorum on the lowest-latency path, and do not let bulk OSD recovery saturate it.
- **Failure-domain reality.** Two leaf switches are not two failure domains if they share power, management, or an upstream link.
