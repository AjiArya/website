+++
title = "Ceph’s data path is simpler than its operations"
date = 2026-10-19
description = "Object clients talk to RGW, RGW talks to the monitor quorum, and the OSDs do the actual work. The operational complexity lives around that path."
template = "blog/page.html"

[taxonomies]
tags = ["ceph", "storage", "architecture"]

[extra]
demo = true
+++
Ceph can feel like a system with too many daemons. The data path, though, is remarkably direct: clients authenticate, receive a cluster map, and then talk to the OSDs that own the objects.

{{<diagram name="ceph-data-path" caption="The Ceph object data path: authentication through MON, placement through the cluster map, and I/O directly to OSDs." />}}

## The parts that matter

The monitor quorum is the source of truth for cluster membership and the cluster map. If the monitors cannot agree, clients cannot get a current view of the cluster. That is why monitor latency and quorum health deserve attention before the first OSD fills up.

The manager is not in the I/O path. It collects telemetry, exposes the dashboard, and runs modules. Losing it should not stop writes; losing the monitor quorum will.

RGW is the object gateway. It authenticates S3 and Swift requests and translates them into RADOS operations. Once a client has a map, reads and writes go to OSDs, not through the gateway again.

## The operational surface

The complexity is in placement, recovery, and failure domains:

- **Placement groups** decide how objects map to OSDs. A poorly planned pool can make recovery slow or impossible to reason about.
- **Failure domains** are only useful if they match reality. A “rack” that shares a power feed is not a failure domain.
- **Recovery** is a first-class workload. A cluster that is healthy at steady state can still be unusable during a rebuild.
- **BlueStore** puts data and metadata on the same device. That is simpler to operate, but capacity planning still needs headroom.

## What I check first

```bash
ceph -s
ceph osd df tree
ceph pg stat
ceph health detail
```

The first command tells me whether the cluster is healthy. The second tells me whether the capacity headroom is real. The third tells me whether recovery is making progress. The fourth tells me whether the health is a warning I can ignore or a symptom of the next outage.

The diagram does not make Ceph simple. It makes the conversation honest: the data path is short, and the operational work is everything around it.
