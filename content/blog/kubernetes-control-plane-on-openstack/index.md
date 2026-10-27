+++
title = "The Kubernetes control plane, as it actually runs on OpenStack"
date = 2026-10-12
description = "A request-path view of Kubernetes on OpenStack: ingress, apiserver, etcd, kubelet, Ceph CSI, and the failure modes between them."
template = "blog/page.html"

[taxonomies]
tags = ["kubernetes", "openstack", "ceph", "architecture"]

[extra]
demo = true
+++
Most Kubernetes diagrams start at the API server. That is a reasonable place to start reading, but it is not where a request starts, and it is not where the interesting failures live.

Here is the path I use when I need to explain a cluster to someone who will operate it:

{{<diagram name="k8s-control-plane" caption="A request path through Kubernetes on OpenStack, ending at Ceph-backed storage." />}}

## Follow the request path

The client speaks HTTPS to an ingress gateway. The gateway terminates TLS and forwards to the API server. The API server authenticates the request, applies admission, and writes the resulting object to etcd.

From there, the control plane is a set of watches and reconciliations. The scheduler binds a pod to a node. The controller manager notices that the desired state and observed state disagree. The kubelet on the chosen node sees the pod, asks the container runtime to start it, and reports status back.

The storage path is separate. The Ceph CSI driver turns a persistent volume claim into a RADOS-backed volume. The kubelet mounts it. The application writes to a filesystem that is actually replicated across OSDs.

## What the diagram leaves out

It leaves out the parts that make production interesting:

- **etcd latency.** A slow etcd is a slow API server. Everything upstream inherits that latency.
- **The network between the control plane and the workers.** Overlay networks, MTU, and security groups are not visible in a box-and-arrow diagram, but they decide whether the line is real.
- **The difference between “the API server is up” and “the cluster is healthy.”** A control plane can accept writes while nothing schedules.
- **Storage recovery.** Ceph may be healthy from Kubernetes' point of view while it is busy rebuilding.

## Three failure modes I watch

1. **A control-plane node loses its etcd member.** The remaining members keep quorum, but the API server on the failed node is now a trap for load balancers that only check TCP.
2. **The CSI driver cannot reach the Ceph monitors.** Pods stay pending, or worse, existing volumes fail to mount on a new node.
3. **A worker has a full disk.** The kubelet evicts pods, but the container runtime and the Ceph OSD may disagree about what “full” means.

The diagram is useful because it gives us a shared vocabulary for those conversations. It is not useful if we pretend the arrows are guarantees.

## A small operational rule

When a request is slow, walk the path once and write down the latency budget at each hop. When a request fails, walk it again and write down the failure domain at each hop. The boxes do not change; the questions do.

```bash
# A quick sanity check from a worker node
curl -sk https://kubernetes.default.svc/healthz
systemctl is-active kubelet containerd
ceph -s
```

The next step is not another dashboard. It is a runbook that says what to check first, what to ignore, and who owns the boundary between the platform and the storage cluster.
