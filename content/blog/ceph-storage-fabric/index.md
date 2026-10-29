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

The logical view is deliberately simpler: it shows which hosts participate in the Ceph public network and which participate in the cluster network. Monitors stay on the public network; OSDs use both.

{{<diagram name="ceph-storage-fabric-simple" caption="Ceph logical networks: the public network connects clients, monitors, and OSDs, while the cluster network carries OSD replication and recovery traffic." />}}

The physical view below expands that into NICs, ports, leaf switches, and individual links.

{{<diagram name="ceph-storage-fabric" caption="Three MON hosts and three OSD hosts, each with two dual-port NICs, dual-homed to two leaf switches. bond0 and bond1 are MLAG bonds; each one has a member on leaf-01 and leaf-02." />}}

## The topology

Six physical hosts sit below a pair of 48-port leaf switches. Each host has two dual-port NICs:

- **NIC 1** has two ports.
- **NIC 2** has two ports.
- Every port is connected: one port from each NIC goes to `leaf-01`, and one port from each NIC goes to `leaf-02`.

That gives every host four independent paths to the fabric, arranged as two two-member bonds:

- **bond0** = `nic1/p1` → `leaf-01` + `nic2/p1` → `leaf-02`
- **bond1** = `nic2/p2` → `leaf-01` + `nic1/p2` → `leaf-02`

Each bond has one member on each NIC and one member on each leaf. A failed NIC, cable, or leaf removes one member from each bond, not a whole bond. The dashed badges and matching colors in the physical diagram mark the two bond groups.

The leaf pair must be configured as an MLAG (or a stacked pair) so both switches present one logical LAG endpoint. Without that, the two bond members on different leaves would not form a single LAG.

## Logical networks

The public network is the client-facing path: clients reach RGW/MON services and the OSDs accept front-end traffic. The cluster network is the private back-end used by OSDs for heartbeat, object replication, and recovery. Ceph Monitors always operate on the public network, so the dedicated MON hosts are shown only on the public rail.

An OSD host is normally on both networks. In the logical diagram, the two colored rails show that membership directly; hovering a legend entry highlights that network's rail and stubs. In the physical diagram, the same host has separate NICs and ports for each path.

## Reading the port grid

Each leaf shows its full 48-port grid. Port numbers 1–12 are in use: six hosts contribute two links each. The highlighted ports are the ones carrying links in this diagram.

On a host, the two NIC blocks show the port-level wiring directly:

- `nic1/1` → `leaf-01`
- `nic1/2` → `leaf-02`
- `nic2/1` → `leaf-02`
- `nic2/2` → `leaf-01`

Port cells and links are colored by bond: bond0 is blue, bond1 is amber. Each bond gets a dashed badge below its host, labeled `bond0` or `bond1`, so the bond grouping is visible without hiding the individual NIC/port mapping. The NIC labels still identify `nic1` and `nic2`; the port numbers identify the member port. Hovering any host highlights its four links and both bond badges, and dims the rest; the diagram is also pan/zoomable for detail.

## What I check first

- **MLAG health** on the leaf pair. A split MLAG is worse than a single switch.
- **Bond member state** on every host. A bond that has silently lost half its members still passes traffic, but no longer has the redundancy you designed.
- **LACP hashing and link utilization.** Four links do not help if one flow can only use one member.
- **Monitor network latency.** Keep the MON quorum on the lowest-latency path, and do not let bulk OSD recovery saturate it.
- **Failure-domain reality.** Two leaf switches are not two failure domains if they share power, management, or an upstream link.
