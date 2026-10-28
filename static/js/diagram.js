(() => {
  "use strict";

  const SPEEDS = [0.25, 0.5, 1, 1.5, 2];
  const DIRECTIONS = ["forward", "reverse", "alternate"];
  const DIRECTION_LABELS = { forward: "→", reverse: "←", alternate: "↔" };
  const DIRECTION_NAMES = { forward: "forward", reverse: "reverse", alternate: "alternating" };
  const TRAIL_SPECS = [
    { opacity: 0.42, width: 3.0 },
    { opacity: 0.26, width: 2.4 },
    { opacity: 0.14, width: 1.8 },
    { opacity: 0.07, width: 1.2 },
  ];
  const REDUCED_MOTION = window.matchMedia("(prefers-reduced-motion: reduce)");
  const MOBILE = window.matchMedia("(max-width: 900px)");
  const MIN_LEGIBLE_SCALE = 0.85;

  const clamp = (value, min, max) => Math.min(Math.max(value, min), max);
  const number = (value, fallback) => {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : fallback;
  };
  const normalizeDirection = (value) =>
    DIRECTIONS.includes(value) ? value : null;
  const speedLabel = (speed) => `${speed}×`;
  const f = (value) => Number(value).toFixed(1);
  const parsePathPoints = (d) => {
    const numbers = (d.match(/-?(?:\d+\.?\d*|\.\d+)(?:e[-+]?\d+)?/gi) || []).map(Number);
    const points = [];
    for (let index = 0; index + 1 < numbers.length; index += 2) {
      points.push({ x: numbers[index], y: numbers[index + 1] });
    }
    return points;
  };
  const pointsToPath = (points) =>
    `M ${points.map((point) => `${f(point.x)} ${f(point.y)}`).join(" L ")}`;

  document.querySelectorAll("[data-diagram]").forEach(init);

  function init(figure) {
    if (figure.dataset.dgReady) return;

    const svg = figure.querySelector("svg");
    const canvas = figure.querySelector("[data-dg-canvas]");
    const toolbar = figure.querySelector("[data-dg-toolbar]");
    if (!svg || !canvas || !toolbar) return;

    const box = svg.viewBox.baseVal;
    if (!box || !box.width || !box.height) return;

    const base = { x: box.x, y: box.y, w: box.width, h: box.height };
    const drawMs = number(svg.dataset.dgDrawMs, 1200);
    const flowMs = number(svg.dataset.dgFlowMs, 2600);
    const pulseMs = 2000;
    const initialDirection =
      normalizeDirection(svg.dataset.dgDirection) || "alternate";
    const autoplay = svg.dataset.dgAutoplay !== "false";

    const state = {
      scale: 1,
      cx: base.x + base.w / 2,
      cy: base.y + base.h / 2,
      fitScale: 1,
      mode: "fit",
      speedIndex: SPEEDS.indexOf(1),
      speed: 1,
      direction: initialDirection,
      userPaused: REDUCED_MOTION.matches || !autoplay,
      autoPaused: false,
      motionOff: false,
      multiSelect: false,
    };

    const playButton = figure.querySelector("[data-dg-play]");
    const playIcon = figure.querySelector("[data-dg-play-icon]");
    const playLabel = figure.querySelector("[data-dg-play-label]");
    const motionButton = figure.querySelector("[data-dg-motion]");
    const multiButton = figure.querySelector("[data-dg-multi]");
    const speedControl = figure.querySelector("[data-dg-speed-control]");
    const speedButton = figure.querySelector("[data-dg-speed]");
    const speedRange = figure.querySelector("[data-dg-speed-range]");
    const speedTicks = Array.from(
      figure.querySelectorAll("[data-dg-speed-tick]"),
    );
    const directionControl = figure.querySelector(
      "[data-dg-direction-control]",
    );
    const directionButton = figure.querySelector("[data-dg-direction]");
    const directionRange = figure.querySelector("[data-dg-direction-range]");
    const directionTicks = Array.from(
      figure.querySelectorAll("[data-dg-direction-tick]"),
    );
    const zoomInButton = figure.querySelector("[data-dg-zoom-in]");
    const zoomOutButton = figure.querySelector("[data-dg-zoom-out]");
    const fitButton = figure.querySelector("[data-dg-fit]");
    const resetButton = figure.querySelector("[data-dg-reset]");
    const pngButton = figure.querySelector("[data-dg-png]");
    const zoomLabel = figure.querySelector("[data-dg-zoom-label]");
    const hint = figure.querySelector("[data-dg-hint]");

    const nodes = Array.from(svg.querySelectorAll(".dg-node"))
      .map((element) => {
        const rect = element.querySelector(".dg-node-box");
        if (!rect) return null;
        return {
          element,
          id: element.dataset.node,
          group: element.dataset.group || "",
          loadBalancer: element.dataset.dgLb === "true",
          lbAlgorithm: element.dataset.dgLbAlgorithm || "round-robin",
          base: {
            x: number(rect.getAttribute("x"), 0),
            y: number(rect.getAttribute("y"), 0),
            w: number(rect.getAttribute("width"), 0),
            h: number(rect.getAttribute("height"), 0),
          },
          tx: 0,
          ty: 0,
        };
      })
      .filter(Boolean);
    const nodeById = new Map(nodes.map((node) => [node.id, node]));

    const groups = Array.from(svg.querySelectorAll(".dg-group"))
      .map((element) => ({
        element,
        id: element.dataset.group,
        box: element.querySelector(".dg-group-box"),
        label: element.querySelector(".dg-group-label"),
      }))
      .filter((group) => group.box && group.label);

    const edges = Array.from(svg.querySelectorAll(".dg-edge")).map(
      (path, index) => ({
        path,
        index,
        from: path.dataset.from,
        to: path.dataset.to,
        route: path.dataset.dgRoute || "elbow",
        direction: normalizeDirection(path.dataset.dgDirection),
        delayMs: number(
          path.style.getPropertyValue("--dg-delay").replace("ms", ""),
          0,
        ),
        points: parsePathPoints(path.getAttribute("d")),
        label: null,
      }),
    );
    svg.querySelectorAll(".dg-edge-label").forEach((label) => {
      const index = number(label.dataset.dgEdge, -1);
      if (edges[index]) edges[index].label = label;
    });

    const packets = Array.from(svg.querySelectorAll(".dg-packet"))
      .map((circle) => {
        const edge = edges[number(circle.dataset.dgEdge, -1)];
        if (!edge) return null;
        const source = nodeById.get(edge.from);
        const loadBalancer = Boolean(source?.loadBalancer);
        return {
          circle,
          edge,
          phase: loadBalancer ? 0 : (edge.delayMs / flowMs) % 1,
          active: !loadBalancer,
          loadBalancer,
          algorithm: source?.lbAlgorithm || "round-robin",
          connections: 0,
          lastDispatch: 0,
          trails: [],
        };
      })
      .filter(Boolean);

    // The tail is drawn as fading segments on the edge itself, so the glow
    // stays on the line rather than following the packet as a blob.
    for (const packet of packets) {
      const group = packet.circle.parentNode;
      const tone = Array.from(packet.circle.classList).find((name) =>
        name.startsWith("dg-tone-"),
      );
      TRAIL_SPECS.forEach((spec, index) => {
        const trail = document.createElementNS(
          "http://www.w3.org/2000/svg",
          "path",
        );
        trail.setAttribute(
          "class",
          `dg-packet-trail${tone ? ` ${tone}` : ""}`,
        );
        trail.setAttribute("fill", "none");
        trail.setAttribute("stroke-linecap", "round");
        trail.setAttribute("stroke-linejoin", "round");
        trail.style.strokeWidth = String(spec.width);
        trail.style.opacity = "0";
        trail.dataset.dgEdge = packet.circle.dataset.dgEdge || "";
        group.insertBefore(trail, packet.circle);
        packet.trails.push({
          path: trail,
          index,
          opacity: spec.opacity,
        });
      });
    }

    const animatedEdges = Array.from(svg.querySelectorAll(".dg-edge")).filter(
      (edge) =>
        edge.classList.contains("dg-edge--draw") ||
        edge.classList.contains("dg-edge--flow") ||
        edge.classList.contains("dg-edge--pulse"),
    );

    const originalNodes = nodes.map((node) =>
      node.element.getAttribute("transform"),
    );
    const originalEdges = edges.map((edge) => ({
      d: edge.path.getAttribute("d"),
      labelX: edge.label ? edge.label.getAttribute("x") : null,
      labelY: edge.label ? edge.label.getAttribute("y") : null,
    }));
    const originalGroups = groups.map((group) => ({
      box: {
        x: group.box.getAttribute("x"),
        y: group.box.getAttribute("y"),
        width: group.box.getAttribute("width"),
        height: group.box.getAttribute("height"),
      },
      labelX: group.label.getAttribute("x"),
      labelY: group.label.getAttribute("y"),
    }));
    const originalPacketPhases = packets.map((packet) => packet.phase);

    const pointers = new Map();
    const selectedNodes = new Set();
    let drag = null;
    let nodeDrag = null;
    let pinch = null;
    let hintHidden = false;
    let onScreen = true;
    let tabHidden = false;
    let packetFrame = 0;
    let packetLast = 0;
    let dispatchers = [];

    function updateAutoPause() {
      state.autoPaused = !onScreen || tabHidden;
      updateAnimations();
    }

    function canvasSize() {
      return {
        w: Math.max(1, canvas.clientWidth),
        h: Math.max(1, canvas.clientHeight),
      };
    }

    function updateFitScale() {
      const { w, h } = canvasSize();
      state.fitScale = Math.min(w / base.w, h / base.h) || 1;
    }

    function setZoomLabel() {
      if (!zoomLabel) return;
      zoomLabel.textContent = `${Math.round((state.scale / state.fitScale) * 100)}%`;
    }

    function applyView() {
      const { w, h } = canvasSize();
      const viewW = w / state.scale;
      const viewH = h / state.scale;

      // Keep an elastic margin so right-drag feels responsive even when the
      // whole diagram is already visible at fit size. This matters for
      // portrait diagrams (like Ceph) that fit by height and have wide side
      // margins: without it, horizontal panning has no range.
      const marginX = viewW * 0.18;
      const marginY = viewH * 0.18;
      let cx = state.cx;
      let cy = state.cy;

      if (viewW >= base.w) {
        const centerX = base.x + base.w / 2;
        cx = clamp(cx, centerX - marginX, centerX + marginX);
      } else {
        cx = clamp(cx, base.x + viewW / 2, base.x + base.w - viewW / 2);
      }
      if (viewH >= base.h) {
        const centerY = base.y + base.h / 2;
        cy = clamp(cy, centerY - marginY, centerY + marginY);
      } else {
        cy = clamp(cy, base.y + viewH / 2, base.y + base.h - viewH / 2);
      }

      state.cx = cx;
      state.cy = cy;
      svg.setAttribute(
        "viewBox",
        `${cx - viewW / 2} ${cy - viewH / 2} ${viewW} ${viewH}`,
      );
      setZoomLabel();
    }

    function fit() {
      updateFitScale();
      state.scale = state.fitScale;
      state.cx = base.x + base.w / 2;
      state.cy = base.y + base.h / 2;
      state.mode = "fit";
      applyView();
    }

    function initialView() {
      updateFitScale();
      const { w, h } = canvasSize();

      if (MOBILE.matches) {
        // Never shrink below a legible label size on a phone. Start at the
        // top-left and let the reader pan from there.
        state.scale = Math.max(state.fitScale, MIN_LEGIBLE_SCALE);
        const viewW = w / state.scale;
        const viewH = h / state.scale;
        state.cx = base.x + Math.min(viewW, base.w) / 2;
        state.cy = base.y + Math.min(viewH, base.h) / 2;
        state.mode = "readable";
        applyView();
      } else {
        fit();
      }
    }

    function hideHint() {
      if (hintHidden || !hint) return;
      hintHidden = true;
      hint.classList.add("is-hidden");
    }

    function updateHintText() {
      if (!hint) return;
      hint.textContent = MOBILE.matches
        ? "Drag to pan · Pinch to zoom · Tap Multi to select nodes · Drag a group"
        : "Right-drag to pan · Shift-click to multi-select · Drag a group to move it";
    }

    function zoomBy(factor, clientX, clientY) {
      const { w, h } = canvasSize();
      const rect = canvas.getBoundingClientRect();
      const px = clamp((clientX - rect.left) / rect.width, 0, 1);
      const py = clamp((clientY - rect.top) / rect.height, 0, 1);
      const oldW = w / state.scale;
      const oldH = h / state.scale;
      const pointX = state.cx - oldW / 2 + px * oldW;
      const pointY = state.cy - oldH / 2 + py * oldH;
      const minScale = state.fitScale * 0.5;
      const maxScale = state.fitScale * 6;
      const nextScale = clamp(state.scale * factor, minScale, maxScale);
      if (Math.abs(nextScale - state.scale) < 0.0001) return;

      const newW = w / nextScale;
      const newH = h / nextScale;
      state.cx = pointX + (0.5 - px) * newW;
      state.cy = pointY + (0.5 - py) * newH;
      state.scale = nextScale;
      state.mode = "manual";
      applyView();
      hideHint();
    }

    function zoomAtCenter(factor) {
      const rect = canvas.getBoundingClientRect();
      zoomBy(factor, rect.left + rect.width / 2, rect.top + rect.height / 2);
    }

    function panByPixels(dx, dy) {
      state.cx -= dx / state.scale;
      state.cy -= dy / state.scale;
      state.mode = "manual";
      applyView();
      hideHint();
    }

    function currentRect(node) {
      return {
        x: node.base.x + node.tx,
        y: node.base.y + node.ty,
        w: node.base.w,
        h: node.base.h,
      };
    }

    function center(rect) {
      return { x: rect.x + rect.w / 2, y: rect.y + rect.h / 2 };
    }

    function anchor(rect, toward) {
      const origin = center(rect);
      const dx = toward.x - origin.x;
      const dy = toward.y - origin.y;
      if (Math.abs(dx) >= Math.abs(dy)) {
        if (dx >= 0) {
          return { x: rect.x + rect.w, y: origin.y, side: "right" };
        }
        return { x: rect.x, y: origin.y, side: "left" };
      }
      if (dy >= 0) {
        return { x: origin.x, y: rect.y + rect.h, side: "bottom" };
      }
      return { x: origin.x, y: rect.y, side: "top" };
    }

    function isHorizontal(side) {
      return side === "left" || side === "right";
    }

    function routeGeometry(from, to, route) {
      const fromCenter = center(from);
      const toCenter = center(to);
      const start = anchor(from, toCenter);
      const end = anchor(to, fromCenter);
      const label = {
        x: (start.x + end.x) / 2,
        y: (start.y + end.y) / 2,
      };

      if (route === "straight") {
        return {
          d: `M ${f(start.x)} ${f(start.y)} L ${f(end.x)} ${f(end.y)}`,
          label,
        };
      }

      if (route === "curve") {
        const dx = end.x - start.x;
        const dy = end.y - start.y;
        let c1;
        let c2;
        if (isHorizontal(start.side) && isHorizontal(end.side)) {
          c1 = { x: start.x + dx * 0.5, y: start.y };
          c2 = { x: end.x - dx * 0.5, y: end.y };
        } else if (!isHorizontal(start.side) && !isHorizontal(end.side)) {
          c1 = { x: start.x, y: start.y + dy * 0.5 };
          c2 = { x: end.x, y: end.y - dy * 0.5 };
        } else if (isHorizontal(start.side)) {
          c1 = { x: start.x + dx * 0.5, y: start.y };
          c2 = { x: end.x, y: end.y - dy * 0.5 };
        } else {
          c1 = { x: start.x, y: start.y + dy * 0.5 };
          c2 = { x: end.x - dx * 0.5, y: end.y };
        }
        return {
          d: `M ${f(start.x)} ${f(start.y)} C ${f(c1.x)} ${f(c1.y)}, ${f(c2.x)} ${f(c2.y)}, ${f(end.x)} ${f(end.y)}`,
          label,
        };
      }

      let points;
      if (isHorizontal(start.side) && isHorizontal(end.side)) {
        const mid = (start.x + end.x) / 2;
        points = [
          start,
          { x: mid, y: start.y },
          { x: mid, y: end.y },
          end,
        ];
      } else if (!isHorizontal(start.side) && !isHorizontal(end.side)) {
        const mid = (start.y + end.y) / 2;
        points = [
          start,
          { x: start.x, y: mid },
          { x: end.x, y: mid },
          end,
        ];
      } else if (isHorizontal(start.side)) {
        points = [start, { x: end.x, y: start.y }, end];
      } else {
        points = [start, { x: start.x, y: end.y }, end];
      }

      let d = `M ${f(points[0].x)} ${f(points[0].y)}`;
      for (const point of points.slice(1)) {
        d += ` L ${f(point.x)} ${f(point.y)}`;
      }
      return { d, label };
    }

    function shiftStart(points, dx, dy) {
      const old = points[0];
      const next = { x: old.x + dx, y: old.y + dy };
      points[0] = next;
      if (points.length < 2) return;
      const second = points[1];
      if (Math.abs(second.x - old.x) < 0.01) {
        second.x = next.x;
      } else if (Math.abs(second.y - old.y) < 0.01) {
        second.y = next.y;
      }
    }

    function shiftEnd(points, dx, dy) {
      const last = points.length - 1;
      const old = points[last];
      const next = { x: old.x + dx, y: old.y + dy };
      points[last] = next;
      if (last < 1) return;
      const previous = points[last - 1];
      if (Math.abs(previous.x - old.x) < 0.01) {
        previous.x = next.x;
      } else if (Math.abs(previous.y - old.y) < 0.01) {
        previous.y = next.y;
      }
    }

    function updateIncidentEdges(movedIds) {
      for (const edge of edges) {
        const fromMoved = movedIds.has(edge.from);
        const toMoved = movedIds.has(edge.to);
        if (!fromMoved && !toMoved) continue;

        const fromNode = nodeById.get(edge.from);
        const toNode = nodeById.get(edge.to);
        if (!fromNode || !toNode) continue;

        // Keep untouched edges byte-for-byte identical. For incident edges,
        // translate the original ports and elbows so the obstacle-aware route
        // survives the drag instead of being rerouted through other nodes.
        const sameOffset =
          Math.abs(fromNode.tx - toNode.tx) < 0.01 &&
          Math.abs(fromNode.ty - toNode.ty) < 0.01;

        let label = null;
        if (fromMoved && toMoved && sameOffset) {
          const points = edge.points.map((point) => ({
            x: point.x + fromNode.tx,
            y: point.y + fromNode.ty,
          }));
          edge.path.setAttribute("d", pointsToPath(points));
        } else if (
          (fromMoved && toMoved) ||
          edge.route !== "elbow" ||
          edge.points.length < 3
        ) {
          const geometry = routeGeometry(
            currentRect(fromNode),
            currentRect(toNode),
            edge.route,
          );
          edge.path.setAttribute("d", geometry.d);
          label = geometry.label;
        } else {
          const points = edge.points.map((point) => ({ ...point }));
          if (fromMoved) {
            shiftStart(points, fromNode.tx, fromNode.ty);
          }
          if (toMoved) {
            shiftEnd(points, toNode.tx, toNode.ty);
          }
          edge.path.setAttribute("d", pointsToPath(points));
        }

        if (edge.label) {
          if (!label) {
            const length = edge.path.getTotalLength();
            const middle = edge.path.getPointAtLength(length / 2);
            label = { x: middle.x, y: middle.y + 7 };
          }
          edge.label.setAttribute("x", f(label.x));
          edge.label.setAttribute("y", f(label.y - 7));
        }
      }
      positionPackets();
    }

    function updateGroups() {
      for (const group of groups) {
        const members = nodes.filter((node) => node.group === group.id);
        if (!members.length) continue;

        let minX = Infinity;
        let minY = Infinity;
        let maxX = -Infinity;
        let maxY = -Infinity;
        for (const node of members) {
          const rect = currentRect(node);
          minX = Math.min(minX, rect.x);
          minY = Math.min(minY, rect.y);
          maxX = Math.max(maxX, rect.x + rect.w);
          maxY = Math.max(maxY, rect.y + rect.h);
        }

        let labelWidth = 0;
        try {
          labelWidth = group.label.getBBox().width;
        } catch (_) {
          labelWidth = (group.label.textContent || "").length * 7;
        }
        const width = Math.max(maxX - minX + 32, labelWidth + 32);
        const x = minX - 16;
        const y = minY - 42;
        group.box.setAttribute("x", f(x));
        group.box.setAttribute("y", f(y));
        group.box.setAttribute("width", f(width));
        group.box.setAttribute("height", f(maxY - minY + 58));
        group.label.setAttribute("x", f(x + 14));
        group.label.setAttribute("y", f(y + 21));
      }
    }

    function directionForEdge(edge) {
      return edge.direction || state.direction;
    }

    function packetProgress(packet) {
      const direction = directionForEdge(packet.edge);
      if (direction === "reverse") return 1 - packet.phase;
      if (direction === "alternate") {
        return packet.phase < 0.5
          ? packet.phase * 2
          : (1 - packet.phase) * 2;
      }
      return packet.phase;
    }

    function trailSegment(path, length, start, end) {
      const from = clamp(start, 0, 1) * length;
      const to = clamp(end, 0, 1) * length;
      const steps = Math.max(2, Math.ceil(Math.abs(to - from) / 5));
      let d = "";
      for (let step = 0; step <= steps; step += 1) {
        const point = path.getPointAtLength(
          from + ((to - from) * step) / steps,
        );
        d += `${step === 0 ? "M" : " L"} ${f(point.x)} ${f(point.y)}`;
      }
      return d;
    }

    function packetVisible(packet) {
      if (state.motionOff) return false;
      return packet.loadBalancer ? packet.active : true;
    }

    function positionPackets() {
      for (const packet of packets) {
        const length = packet.edge.path.getTotalLength();
        const visible = packetVisible(packet);
        packet.circle.style.opacity = visible ? "1" : "0";
        if (!length) continue;

        const progress = packetProgress(packet);
        const point = packet.edge.path.getPointAtLength(progress * length);
        packet.circle.setAttribute("cx", f(point.x));
        packet.circle.setAttribute("cy", f(point.y));

        const direction = directionForEdge(packet.edge);
        let movingForward = true;
        if (direction === "reverse") {
          movingForward = false;
        } else if (direction === "alternate") {
          movingForward = packet.phase < 0.5;
        }
        const segment = Math.min(26, Math.max(8, length * 0.06)) / length;
        const gap = Math.min(8, Math.max(3, length * 0.03)) / length;

        for (const trail of packet.trails) {
          let start;
          let end;
          if (movingForward) {
            end = progress - trail.index * (segment + gap);
            start = end - segment;
          } else {
            start = progress + trail.index * (segment + gap);
            end = start + segment;
          }

          if (!visible || start < 0 || end > 1) {
            trail.path.style.opacity = "0";
            continue;
          }

          trail.path.setAttribute(
            "d",
            trailSegment(packet.edge.path, length, start, end),
          );
          trail.path.style.opacity = String(trail.opacity);
        }
      }
    }

    function buildDispatchers() {
      const bySource = new Map();
      for (const packet of packets) {
        if (!packet.loadBalancer) continue;
        const source = packet.edge.from || "";
        if (!bySource.has(source)) bySource.set(source, []);
        bySource.get(source).push(packet);
      }
      dispatchers = Array.from(bySource.values()).map((members) => ({
        packets: members,
        algorithm: members[0].algorithm,
        cursor: 0,
        nextAt: 0,
      }));
    }

    function resetPacketState() {
      packets.forEach((packet, index) => {
        packet.phase = packet.loadBalancer ? 0 : originalPacketPhases[index];
        packet.active = !packet.loadBalancer;
        packet.connections = 0;
        packet.lastDispatch = 0;
      });
      dispatchers.forEach((dispatcher) => {
        dispatcher.cursor = 0;
        dispatcher.nextAt = 0;
      });
    }

    function dispatchPacket(dispatcher, now) {
      const idle = dispatcher.packets.filter((packet) => !packet.active);
      if (!idle.length) {
        dispatcher.nextAt = now + 120;
        return;
      }

      let packet = null;
      if (dispatcher.algorithm === "random") {
        packet = idle[Math.floor(Math.random() * idle.length)];
      } else if (dispatcher.algorithm === "least-connections") {
        packet = idle.reduce((best, candidate) => {
          const bestLoad = best.connections || 0;
          const candidateLoad = candidate.connections || 0;
          if (candidateLoad < bestLoad) return candidate;
          if (
            candidateLoad === bestLoad &&
            (candidate.lastDispatch || 0) < (best.lastDispatch || 0)
          ) {
            return candidate;
          }
          return best;
        }, idle[0]);
      } else {
        const total = dispatcher.packets.length;
        for (let step = 0; step < total; step += 1) {
          const index = (dispatcher.cursor + step) % total;
          const candidate = dispatcher.packets[index];
          if (!candidate.active) {
            packet = candidate;
            dispatcher.cursor = (index + 1) % total;
            break;
          }
        }
      }

      if (!packet) {
        dispatcher.nextAt = now + 120;
        return;
      }

      packet.active = true;
      packet.connections = (packet.connections || 0) + 1;
      packet.lastDispatch = now;
      packet.phase = 0;

      const interval = Math.max(
        280,
        flowMs / Math.max(1, dispatcher.packets.length),
      );
      dispatcher.nextAt = now + interval / state.speed;
    }

    function packetRunning() {
      return (
        !state.motionOff &&
        !state.userPaused &&
        !state.autoPaused &&
        packets.length > 0
      );
    }

    function packetTick(now) {
      if (!packetRunning()) {
        packetFrame = 0;
        return;
      }
      const deltaTime = Math.min(64, Math.max(0, now - packetLast));
      packetLast = now;
      const delta = deltaTime / (flowMs / state.speed);

      for (const dispatcher of dispatchers) {
        if (now >= dispatcher.nextAt) {
          dispatchPacket(dispatcher, now);
        }
      }
      for (const packet of packets) {
        if (!packet.loadBalancer) {
          packet.phase = (packet.phase + delta) % 1;
          continue;
        }
        if (!packet.active) continue;
        packet.phase += delta;
        if (packet.phase >= 1) {
          packet.phase = 0;
          packet.active = false;
          packet.connections = Math.max(
            0,
            (packet.connections || 0) - 1,
          );
        }
      }

      positionPackets();
      packetFrame = window.requestAnimationFrame(packetTick);
    }

    function syncPackets() {
      if (packetRunning()) {
        if (!packetFrame) {
          packetLast = window.performance.now();
          packetFrame = window.requestAnimationFrame(packetTick);
        }
      } else {
        if (packetFrame) {
          window.cancelAnimationFrame(packetFrame);
          packetFrame = 0;
        }
        positionPackets();
      }
    }

    function updatePlayButton() {
      if (!playButton) return;
      playButton.disabled = state.motionOff;
      if (state.motionOff) {
        if (playIcon) playIcon.textContent = "■";
        if (playLabel) playLabel.textContent = "Off";
        playButton.setAttribute("aria-label", "Animation is disabled");
        playButton.setAttribute("title", "Animation is disabled");
        return;
      }
      const paused = state.userPaused;
      if (playIcon) playIcon.textContent = paused ? "▶" : "❚❚";
      if (playLabel) playLabel.textContent = paused ? "Play" : "Pause";
      const label = paused ? "Play animation" : "Pause animation";
      playButton.setAttribute("aria-label", label);
      playButton.setAttribute("title", label);
    }

    function updateSpeedLabel() {
      if (!speedButton) return;
      speedButton.textContent = speedLabel(state.speed);
      speedButton.setAttribute(
        "aria-label",
        `Animation speed: ${state.speed}x`,
      );
      if (speedRange) speedRange.value = String(state.speedIndex);
      speedTicks.forEach((tick, index) => {
        tick.classList.toggle("is-active", index === state.speedIndex);
      });
    }

    function updateDirectionLabel() {
      if (!directionButton) return;
      directionButton.textContent = DIRECTION_LABELS[state.direction];
      directionButton.setAttribute(
        "aria-label",
        `Flow direction: ${DIRECTION_NAMES[state.direction]}`,
      );
      const index = DIRECTIONS.indexOf(state.direction);
      if (directionRange) directionRange.value = String(index);
      directionTicks.forEach((tick, tickIndex) => {
        tick.classList.toggle("is-active", tickIndex === index);
      });
    }

    function edgeBaseDuration(edge) {
      if (edge.classList.contains("dg-edge--draw")) return drawMs;
      if (edge.classList.contains("dg-edge--pulse")) return pulseMs;
      return 0;
    }

    function updateMotionButton() {
      if (!motionButton) return;
      motionButton.textContent = state.motionOff ? "Enable" : "Disable";
      motionButton.setAttribute(
        "aria-label",
        state.motionOff ? "Enable animation" : "Disable animation",
      );
      motionButton.setAttribute(
        "title",
        state.motionOff ? "Enable animation" : "Disable animation",
      );
      motionButton.setAttribute("aria-pressed", String(state.motionOff));
    }

    function updateAnimations() {
      const running =
        !state.motionOff && !state.userPaused && !state.autoPaused;
      animatedEdges.forEach((edge) => {
        if (edge.classList.contains("dg-edge--flow")) {
          edge.style.animation = "";
          return;
        }
        const baseDuration = edgeBaseDuration(edge);
        if (!baseDuration) return;

        if (state.motionOff) {
          edge.style.animation = "none";
          edge.dataset.dgFrozen = "true";
          if (edge.classList.contains("dg-edge--draw")) {
            edge.style.strokeDashoffset = "0";
            edge.style.strokeDasharray = "1";
          } else {
            edge.style.strokeOpacity = "1";
          }
          return;
        }

        if (edge.dataset.dgFrozen) {
          if (!running) return;
          edge.style.animation = "";
          edge.style.strokeDashoffset = "";
          edge.style.strokeDasharray = "";
          edge.style.strokeOpacity = "";
          delete edge.dataset.dgFrozen;
        }

        edge.style.animationDuration = `${baseDuration / state.speed}ms`;
        edge.style.animationPlayState = running ? "running" : "paused";
      });
      syncPackets();
      updatePlayButton();
      updateMotionButton();
    }

    function togglePlay() {
      if (state.motionOff) return;
      state.userPaused = !state.userPaused;
      updateAnimations();
    }

    function toggleMotion() {
      state.motionOff = !state.motionOff;
      updateAnimations();
    }

    function resetLayout() {
      nodeDrag = null;
      drag = null;
      pinch = null;
      pointers.clear();
      selectedNodes.clear();
      updateSelectionStyles();
      canvas.classList.remove("is-dragging");

      nodes.forEach((node, index) => {
        node.tx = 0;
        node.ty = 0;
        node.element.classList.remove("is-moving");
        const transform = originalNodes[index];
        if (transform) {
          node.element.setAttribute("transform", transform);
        } else {
          node.element.removeAttribute("transform");
        }
      });

      edges.forEach((edge, index) => {
        const original = originalEdges[index];
        edge.path.setAttribute("d", original.d);
        if (edge.label) {
          if (original.labelX !== null) {
            edge.label.setAttribute("x", original.labelX);
          }
          if (original.labelY !== null) {
            edge.label.setAttribute("y", original.labelY);
          }
        }
      });

      groups.forEach((group, index) => {
        const original = originalGroups[index];
        group.box.setAttribute("x", original.box.x);
        group.box.setAttribute("y", original.box.y);
        group.box.setAttribute("width", original.box.width);
        group.box.setAttribute("height", original.box.height);
        group.label.setAttribute("x", original.labelX);
        group.label.setAttribute("y", original.labelY);
      });

      packets.forEach((packet, index) => {
        packet.phase = originalPacketPhases[index];
      });

      state.speedIndex = SPEEDS.indexOf(1);
      state.speed = 1;
      state.direction = initialDirection;
      state.userPaused = REDUCED_MOTION.matches || !autoplay;
      state.autoPaused = !onScreen || tabHidden;
      state.motionOff = false;
      resetPacketState();

      updateSpeedLabel();
      updateDirectionLabel();
      initialView();
      positionPackets();
      updateAnimations();

      if (hint) {
        hintHidden = false;
        hint.hidden = false;
        hint.classList.remove("is-hidden");
      }

      // Restart draw-in animations from the beginning, matching a fresh load.
      if (!state.motionOff) {
        animatedEdges.forEach((edge) => {
          if (!edge.classList.contains("dg-edge--draw")) return;
          edge.style.animation = "none";
          void edge.getBoundingClientRect();
          edge.style.animation = "";
        });
      }
      updateAnimations();
    }

    function buildDiagramExport() {
      const clone = svg.cloneNode(true);
      const originals = [svg, ...svg.querySelectorAll("*")];
      const clones = [clone, ...clone.querySelectorAll("*")];
      const props = [
        "fill",
        "fill-opacity",
        "stroke",
        "stroke-width",
        "stroke-opacity",
        "stroke-dasharray",
        "stroke-dashoffset",
        "stroke-linecap",
        "stroke-linejoin",
        "opacity",
        "filter",
        "font-family",
        "font-size",
        "font-weight",
        "font-style",
        "letter-spacing",
        "text-anchor",
        "text-transform",
        "dominant-baseline",
        "paint-order",
        "visibility",
        "display",
      ];

      for (let index = 0; index < originals.length; index += 1) {
        const computed = window.getComputedStyle(originals[index]);
        const declarations = props
          .map((prop) => `${prop}:${computed.getPropertyValue(prop)}`)
          .join(";");
        clones[index].setAttribute("style", declarations);
      }

      // A PNG is a static artifact: remove animated packets and freeze every
      // draw/pulse edge at its completed state.
      clone.querySelectorAll("*").forEach((element) => {
        element.style.animation = "none";
        element.style.transition = "none";
      });
      clone.querySelectorAll(".dg-edge--draw").forEach((edge) => {
        edge.style.strokeDashoffset = "0";
        edge.style.strokeDasharray = "1";
      });
      clone.querySelectorAll(".dg-edge--pulse").forEach((edge) => {
        edge.style.strokeOpacity = "1";
      });
      clone.querySelectorAll(".dg-packet, .dg-packet-trail").forEach((packet) => {
        packet.remove();
      });

      const background =
        window
          .getComputedStyle(document.documentElement)
          .getPropertyValue("--bg")
          .trim() || "#0b0d0c";
      const backgroundRect = document.createElementNS(
        "http://www.w3.org/2000/svg",
        "rect",
      );
      backgroundRect.setAttribute("x", String(base.x));
      backgroundRect.setAttribute("y", String(base.y));
      backgroundRect.setAttribute("width", String(base.w));
      backgroundRect.setAttribute("height", String(base.h));
      backgroundRect.setAttribute("fill", background);

      const firstGraphic =
        clone.querySelector(".dg-groups") || clone.querySelector("g");
      if (firstGraphic) {
        clone.insertBefore(backgroundRect, firstGraphic);
      } else {
        clone.appendChild(backgroundRect);
      }

      clone.setAttribute("xmlns", "http://www.w3.org/2000/svg");
      clone.setAttribute("viewBox", `${base.x} ${base.y} ${base.w} ${base.h}`);
      clone.setAttribute("width", String(base.w));
      clone.setAttribute("height", String(base.h));

      return {
        markup: new XMLSerializer().serializeToString(clone),
        background,
        caption:
          figure.querySelector("figcaption")?.textContent?.trim() || "",
        source: figure.dataset.dgSource || "@ajiarya",
      };
    }

    function ellipsize(context, text, maxWidth) {
      if (context.measureText(text).width <= maxWidth) return text;
      let low = 0;
      let high = text.length;
      while (low < high) {
        const middle = Math.ceil((low + high) / 2);
        const candidate = `${text.slice(0, middle).trimEnd()}…`;
        if (context.measureText(candidate).width <= maxWidth) {
          low = middle;
        } else {
          high = middle - 1;
        }
      }
      return `${text.slice(0, low).trimEnd()}…`;
    }

    function renderDiagramPng() {
      return new Promise((resolve, reject) => {
        const exported = buildDiagramExport();
        const svgBlob = new Blob([exported.markup], {
          type: "image/svg+xml;charset=utf-8",
        });
        const url = URL.createObjectURL(svgBlob);
        const image = new Image();
        image.onload = () => {
          const scale = Math.max(
            1,
            Math.min(2.5, 4096 / Math.max(base.w, base.h)),
          );
          const caption = exported.caption;
          const source = exported.source;
          const footer = caption || source ? 38 : 0;
          const canvas = document.createElement("canvas");
          canvas.width = Math.round(base.w * scale);
          canvas.height = Math.round((base.h + footer) * scale);
          const context = canvas.getContext("2d");
          context.fillStyle = exported.background;
          context.fillRect(0, 0, canvas.width, canvas.height);
          context.drawImage(image, 0, 0, canvas.width, Math.round(base.h * scale));
          URL.revokeObjectURL(url);

          const styles = window.getComputedStyle(document.documentElement);
          const color = (name, fallback) =>
            styles.getPropertyValue(name).trim() || fallback;
          const fontSans =
            color("--font-sans", "ui-sans-serif, system-ui, sans-serif");
          const inkSoft = color("--ink-soft", "#9aab9a");
          const muted = color("--ink-muted", "#788579");

          const padding = 20 * scale;
          const baseline = (base.h + 22) * scale;
          const creditSize = 9;
          context.font = `500 ${Math.max(1, creditSize * scale)}px ${fontSans}`;
          const creditWidth = source ? context.measureText(source).width : 0;
          const gap = source ? 12 * scale : 0;
          const captionWidth = Math.max(
            1,
            canvas.width - padding * 2 - creditWidth - gap,
          );

          let captionSize = 11.5;
          let captionText = caption;
          if (caption) {
            const setCaptionFont = () => {
              context.font = `500 ${Math.max(
                1,
                captionSize * scale,
              )}px ${fontSans}`;
            };
            setCaptionFont();
            while (
              captionSize > 7.5 &&
              context.measureText(caption).width > captionWidth
            ) {
              captionSize -= 0.25;
              setCaptionFont();
            }
            if (context.measureText(caption).width > captionWidth) {
              captionText = ellipsize(context, caption, captionWidth);
            }
          }

          if (caption) {
            context.fillStyle = inkSoft;
            context.font = `500 ${Math.max(
              1,
              captionSize * scale,
            )}px ${fontSans}`;
            context.textAlign = "left";
            context.fillText(captionText, padding, baseline);
          }
          if (source) {
            context.fillStyle = muted;
            context.font = `500 ${Math.max(
              1,
              creditSize * scale,
            )}px ${fontSans}`;
            context.textAlign = "right";
            context.fillText(source, canvas.width - padding, baseline);
            context.textAlign = "left";
          }

          canvas.toBlob((blob) => {
            if (blob) {
              resolve(blob);
            } else {
              reject(new Error("Could not encode PNG"));
            }
          }, "image/png");
        };
        image.onerror = () => {
          URL.revokeObjectURL(url);
          reject(new Error("Could not rasterize diagram"));
        };
        image.src = url;
      });
    }

    function downloadPng(blob) {
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = `${figure.dataset.diagramName || "diagram"}.png`;
      document.body.appendChild(anchor);
      anchor.click();
      anchor.remove();
      window.setTimeout(() => URL.revokeObjectURL(url), 1000);
    }

    async function copyDiagramPng() {
      if (!pngButton) return;
      const originalLabel = pngButton.textContent;
      pngButton.disabled = true;
      pngButton.textContent = "…";
      pngButton.setAttribute("aria-label", "Preparing PNG");

      try {
        const blob = await renderDiagramPng();
        let copied = false;
        if (navigator.clipboard && window.ClipboardItem) {
          try {
            await navigator.clipboard.write([
              new ClipboardItem({ "image/png": blob }),
            ]);
            copied = true;
          } catch (_) {
            copied = false;
          }
        }
        if (!copied) downloadPng(blob);
        pngButton.textContent = copied ? "Copied" : "Saved";
        pngButton.setAttribute(
          "aria-label",
          copied ? "PNG copied to clipboard" : "PNG downloaded",
        );
      } catch (error) {
        console.error("Diagram PNG export failed", error);
        pngButton.textContent = "Retry";
        pngButton.setAttribute("aria-label", "PNG export failed; try again");
      } finally {
        pngButton.disabled = false;
        window.setTimeout(() => {
          pngButton.textContent = originalLabel;
          pngButton.setAttribute("aria-label", "Copy diagram as PNG");
        }, 1800);
      }
    }

    function startPinch() {
      const points = Array.from(pointers.values());
      if (points.length < 2) return;
      const [a, b] = points;
      const rect = canvas.getBoundingClientRect();
      const midX = (a.x + b.x) / 2;
      const midY = (a.y + b.y) / 2;
      const px = clamp((midX - rect.left) / rect.width, 0, 1);
      const py = clamp((midY - rect.top) / rect.height, 0, 1);
      const viewW = canvas.clientWidth / state.scale;
      const viewH = canvas.clientHeight / state.scale;
      pinch = {
        startDist: Math.hypot(b.x - a.x, b.y - a.y) || 1,
        startScale: state.scale,
        anchorX: state.cx - viewW / 2 + px * viewW,
        anchorY: state.cy - viewH / 2 + py * viewH,
      };
    }

    function resetPointers() {
      if (pointers.size === 0) {
        drag = null;
        pinch = null;
        canvas.classList.remove("is-dragging");
      } else if (pointers.size === 1) {
        const [id, point] = Array.from(pointers.entries())[0];
        drag = { id, x: point.x, y: point.y, cx: state.cx, cy: state.cy };
        pinch = null;
      } else {
        startPinch();
      }
    }

    function closestNode(element) {
      let current = element;
      while (current && current !== canvas) {
        if (
          current.classList &&
          current.classList.contains("dg-node")
        ) {
          return current;
        }
        current = current.parentNode;
      }
      return null;
    }

    function closestGroup(element) {
      let current = element;
      while (current && current !== canvas) {
        if (
          current.classList &&
          current.classList.contains("dg-group")
        ) {
          return current;
        }
        current = current.parentNode;
      }
      return null;
    }

    function updateSelectionStyles() {
      for (const node of nodes) {
        node.element.classList.toggle(
          "is-selected",
          selectedNodes.has(node.id),
        );
      }
      for (const group of groups) {
        const members = nodes.filter((node) => node.group === group.id);
        const selected =
          members.length > 0 &&
          members.every((node) => selectedNodes.has(node.id));
        group.element.classList.toggle("is-selected", selected);
      }
    }

    function selectionList() {
      return nodes.filter((node) => selectedNodes.has(node.id));
    }

    function updateMultiButton() {
      if (!multiButton) return;
      multiButton.setAttribute("aria-pressed", String(state.multiSelect));
      multiButton.setAttribute(
        "title",
        state.multiSelect
          ? "Multi-select is on; tap nodes to add or remove"
          : "Toggle multi-select; tap nodes to add or remove",
      );
    }

    function startPan(event) {
      drag = {
        id: event.pointerId,
        x: event.clientX,
        y: event.clientY,
        cx: state.cx,
        cy: state.cy,
      };
      canvas.classList.add("is-dragging");
    }

    function startNodeDrag(element, event) {
      const node = nodeById.get(element.dataset.node);
      if (!node) return false;

      const toggle =
        event.shiftKey ||
        event.ctrlKey ||
        event.metaKey ||
        state.multiSelect;
      const wasSelected = selectedNodes.has(node.id);

      if (toggle) {
        if (!wasSelected) selectedNodes.add(node.id);
      } else if (!wasSelected) {
        selectedNodes.clear();
        selectedNodes.add(node.id);
      }
      updateSelectionStyles();

      const moving = selectionList();
      if (!moving.length) {
        selectedNodes.add(node.id);
        moving.push(node);
      }

      nodeDrag = {
        id: event.pointerId,
        focus: node,
        nodes: moving.map((item) => ({
          node: item,
          startTx: item.tx,
          startTy: item.ty,
        })),
        startX: event.clientX,
        startY: event.clientY,
        moved: false,
        toggle: toggle ? node : null,
        wasSelected,
        collapse: !toggle && wasSelected && selectedNodes.size > 1,
      };
      for (const item of nodeDrag.nodes) {
        item.node.element.classList.add("is-moving");
      }
      return true;
    }

    function startGroupDrag(element, event) {
      const group = groups.find((item) => item.id === element.dataset.group);
      if (!group) return false;
      const members = nodes.filter((node) => node.group === group.id);
      if (!members.length) return false;

      const add =
        event.shiftKey ||
        event.ctrlKey ||
        event.metaKey ||
        state.multiSelect;
      if (!add) selectedNodes.clear();
      for (const member of members) selectedNodes.add(member.id);
      updateSelectionStyles();

      nodeDrag = {
        id: event.pointerId,
        focus: null,
        nodes: selectionList().map((node) => ({
          node,
          startTx: node.tx,
          startTy: node.ty,
        })),
        startX: event.clientX,
        startY: event.clientY,
        moved: false,
        toggle: null,
        wasSelected: false,
        collapse: false,
      };
      for (const item of nodeDrag.nodes) {
        item.node.element.classList.add("is-moving");
      }
      return true;
    }

    // Animation controls
    playButton?.addEventListener("click", togglePlay);
    motionButton?.addEventListener("click", toggleMotion);
    multiButton?.addEventListener("click", () => {
      state.multiSelect = !state.multiSelect;
      updateMultiButton();
    });

    speedButton?.addEventListener("click", () => {
      directionControl?.classList.remove("is-open");
      speedControl?.classList.add("is-open");
      window.requestAnimationFrame(() => speedRange?.focus());
    });

    speedRange?.addEventListener("input", () => {
      const index = clamp(
        number(speedRange.value, state.speedIndex),
        0,
        SPEEDS.length - 1,
      );
      state.speedIndex = index;
      state.speed = SPEEDS[index];
      updateSpeedLabel();
      updateAnimations();
    });

    directionButton?.addEventListener("click", () => {
      speedControl?.classList.remove("is-open");
      directionControl?.classList.add("is-open");
      window.requestAnimationFrame(() => directionRange?.focus());
    });

    directionRange?.addEventListener("input", () => {
      const index = clamp(
        number(directionRange.value, DIRECTIONS.indexOf(state.direction)),
        0,
        DIRECTIONS.length - 1,
      );
      state.direction = DIRECTIONS[index];
      updateDirectionLabel();
      updateAnimations();
      positionPackets();
    });

    speedControl?.addEventListener("mouseenter", () => {
      directionControl?.classList.remove("is-open");
    });
    directionControl?.addEventListener("mouseenter", () => {
      speedControl?.classList.remove("is-open");
    });

    document.addEventListener("pointerdown", (event) => {
      if (speedControl && !speedControl.contains(event.target)) {
        speedControl.classList.remove("is-open");
      }
      if (directionControl && !directionControl.contains(event.target)) {
        directionControl.classList.remove("is-open");
      }
    });

    // View controls
    zoomInButton?.addEventListener("click", () => zoomAtCenter(1.25));
    zoomOutButton?.addEventListener("click", () => zoomAtCenter(1 / 1.25));
    fitButton?.addEventListener("click", fit);
    resetButton?.addEventListener("click", resetLayout);
    pngButton?.addEventListener("click", copyDiagramPng);

    // Pointer pan, node move, and pinch
    canvas.addEventListener("pointerdown", (event) => {
      const isMouse = event.pointerType === "mouse";
      if (isMouse && event.button !== 0 && event.button !== 2) return;

      canvas.focus({ preventScroll: true });
      canvas.setPointerCapture(event.pointerId);
      pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });

      if (pointers.size === 1) {
        const left = !isMouse || event.button === 0;
        const nodeElement = left ? closestNode(event.target) : null;
        if (nodeElement && startNodeDrag(nodeElement, event)) {
          event.preventDefault();
          return;
        }

        const groupElement = left ? closestGroup(event.target) : null;
        if (groupElement && startGroupDrag(groupElement, event)) {
          event.preventDefault();
          return;
        }

        if (
          left &&
          !event.shiftKey &&
          !event.ctrlKey &&
          !event.metaKey &&
          !state.multiSelect
        ) {
          selectedNodes.clear();
          updateSelectionStyles();
        }
        startPan(event);
      } else {
        if (nodeDrag) {
          for (const item of nodeDrag.nodes) {
            item.node.element.classList.remove("is-moving");
          }
          nodeDrag = null;
        }
        drag = null;
        startPinch();
      }
      event.preventDefault();
    });

    canvas.addEventListener("pointermove", (event) => {
      if (!pointers.has(event.pointerId)) return;
      pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });

      if (nodeDrag && nodeDrag.id === event.pointerId) {
        const screenDx = event.clientX - nodeDrag.startX;
        const screenDy = event.clientY - nodeDrag.startY;
        if (!nodeDrag.moved) {
          if (Math.hypot(screenDx, screenDy) < 3) {
            event.preventDefault();
            return;
          }
          nodeDrag.moved = true;
        }

        const dx = screenDx / state.scale;
        const dy = screenDy / state.scale;
        const movedIds = new Set();
        for (const item of nodeDrag.nodes) {
          item.node.tx = item.startTx + dx;
          item.node.ty = item.startTy + dy;
          item.node.element.setAttribute(
            "transform",
            `translate(${f(item.node.tx)} ${f(item.node.ty)})`,
          );
          movedIds.add(item.node.id);
        }
        updateIncidentEdges(movedIds);
        updateGroups();
        hideHint();
        event.preventDefault();
        return;
      }

      if (pinch && pointers.size >= 2) {
        const points = Array.from(pointers.values());
        const [a, b] = points;
        const rect = canvas.getBoundingClientRect();
        const midX = (a.x + b.x) / 2;
        const midY = (a.y + b.y) / 2;
        const px = clamp((midX - rect.left) / rect.width, 0, 1);
        const py = clamp((midY - rect.top) / rect.height, 0, 1);
        const dist = Math.hypot(b.x - a.x, b.y - a.y) || 1;
        const nextScale = clamp(
          pinch.startScale * (dist / pinch.startDist),
          state.fitScale * 0.5,
          state.fitScale * 6,
        );
        const { w, h } = canvasSize();
        const newW = w / nextScale;
        const newH = h / nextScale;
        state.cx = pinch.anchorX + (0.5 - px) * newW;
        state.cy = pinch.anchorY + (0.5 - py) * newH;
        state.scale = nextScale;
        state.mode = "manual";
        applyView();
        hideHint();
        event.preventDefault();
        return;
      }

      if (!drag || drag.id !== event.pointerId) return;
      state.cx = drag.cx - (event.clientX - drag.x) / state.scale;
      state.cy = drag.cy - (event.clientY - drag.y) / state.scale;
      state.mode = "manual";
      applyView();
      hideHint();
      event.preventDefault();
    });

    const endPointer = (event) => {
      if (nodeDrag && nodeDrag.id === event.pointerId) {
        if (!nodeDrag.moved) {
          if (nodeDrag.toggle) {
            if (nodeDrag.wasSelected) {
              selectedNodes.delete(nodeDrag.toggle.id);
            } else {
              selectedNodes.add(nodeDrag.toggle.id);
            }
          } else if (nodeDrag.collapse && nodeDrag.focus) {
            selectedNodes.clear();
            selectedNodes.add(nodeDrag.focus.id);
          }
        }
        for (const item of nodeDrag.nodes) {
          item.node.element.classList.remove("is-moving");
        }
        nodeDrag = null;
        updateSelectionStyles();
      }
      if (!pointers.has(event.pointerId)) return;
      pointers.delete(event.pointerId);
      resetPointers();
    };
    canvas.addEventListener("pointerup", endPointer);
    canvas.addEventListener("pointercancel", endPointer);
    canvas.addEventListener("lostpointercapture", endPointer);

    // A normal wheel over the canvas zooms the diagram. Moving the pointer
    // outside the canvas keeps the page scroll behaviour intact.
    canvas.addEventListener(
      "wheel",
      (event) => {
        event.preventDefault();
        zoomBy(
          Math.exp(-event.deltaY * 0.0015),
          event.clientX,
          event.clientY,
        );
      },
      { passive: false },
    );

    // Right-drag should pan, not open the browser menu.
    canvas.addEventListener("contextmenu", (event) => {
      event.preventDefault();
    });

    canvas.addEventListener("dblclick", fit);

    canvas.addEventListener("keydown", (event) => {
      const { w, h } = canvasSize();
      const panX = w * 0.12;
      const panY = h * 0.12;
      switch (event.key) {
        case "+":
        case "=":
          zoomAtCenter(1.2);
          event.preventDefault();
          break;
        case "-":
        case "_":
          zoomAtCenter(1 / 1.2);
          event.preventDefault();
          break;
        case "0":
        case "f":
          fit();
          event.preventDefault();
          break;
        case "ArrowLeft":
          panByPixels(-panX, 0);
          event.preventDefault();
          break;
        case "ArrowRight":
          panByPixels(panX, 0);
          event.preventDefault();
          break;
        case "ArrowUp":
          panByPixels(0, -panY);
          event.preventDefault();
          break;
        case "ArrowDown":
          panByPixels(0, panY);
          event.preventDefault();
          break;
        case " ":
          togglePlay();
          event.preventDefault();
          break;
        case "Escape":
          if (selectedNodes.size) {
            selectedNodes.clear();
            updateSelectionStyles();
            event.preventDefault();
          }
          break;
        default:
          break;
      }
    });

    if ("IntersectionObserver" in window) {
      const observer = new IntersectionObserver(
        (entries) => {
          onScreen = entries[entries.length - 1].isIntersecting;
          updateAutoPause();
        },
        { threshold: 0.05 },
      );
      observer.observe(figure);
    }

    document.addEventListener("visibilitychange", () => {
      tabHidden = document.hidden;
      updateAutoPause();
    });

    REDUCED_MOTION.addEventListener("change", (event) => {
      if (event.matches) {
        state.userPaused = true;
        updateAnimations();
      }
    });

    let resizeFrame = 0;
    if ("ResizeObserver" in window) {
      const resizeObserver = new ResizeObserver(() => {
        window.cancelAnimationFrame(resizeFrame);
        resizeFrame = window.requestAnimationFrame(() => {
          const previousFit = state.fitScale;
          updateFitScale();
          if (state.mode === "fit" || state.mode === "readable") {
            initialView();
          } else if (previousFit > 0) {
            state.scale = (state.scale / previousFit) * state.fitScale;
            applyView();
          }
        });
      });
      resizeObserver.observe(canvas);
    }

    updateSpeedLabel();
    updateDirectionLabel();
    updateMultiButton();
    updateHintText();
    MOBILE.addEventListener("change", updateHintText);

    window.requestAnimationFrame(() => {
      buildDispatchers();
      initialView();
      positionPackets();
      updateAnimations();
      toolbar.hidden = false;
      if (hint) hint.hidden = false;
      figure.setAttribute("data-dg-ready", "");
    });
  }
})();
