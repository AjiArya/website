(() => {
  "use strict";

  const SPEEDS = [0.5, 1, 1.5, 2];
  const DIRECTIONS = ["forward", "reverse", "alternate"];
  const DIRECTION_LABELS = { forward: "→", reverse: "←", alternate: "⇄" };
  const DIRECTION_NAMES = { forward: "forward", reverse: "reverse", alternate: "alternating" };
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
    const initialDirection = normalizeDirection(svg.dataset.dgDirection) || "forward";
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
    };

    const playButton = figure.querySelector("[data-dg-play]");
    const playIcon = figure.querySelector("[data-dg-play-icon]");
    const playLabel = figure.querySelector("[data-dg-play-label]");
    const speedButton = figure.querySelector("[data-dg-speed]");
    const directionButton = figure.querySelector("[data-dg-direction]");
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
        return {
          circle,
          edge,
          phase: (edge.delayMs / flowMs) % 1,
        };
      })
      .filter(Boolean);

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
    let drag = null;
    let nodeDrag = null;
    let pinch = null;
    let hintHidden = false;
    let onScreen = true;
    let tabHidden = false;
    let packetFrame = 0;
    let packetLast = 0;

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
        ? "Drag to pan · Pinch to zoom · Drag a node to move"
        : "Right-drag to pan · Scroll to zoom · Drag a node to move";
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

    function updateEdges() {
      for (const edge of edges) {
        const fromNode = nodeById.get(edge.from);
        const toNode = nodeById.get(edge.to);
        if (!fromNode || !toNode) continue;
        const geometry = routeGeometry(
          currentRect(fromNode),
          currentRect(toNode),
          edge.route,
        );
        edge.path.setAttribute("d", geometry.d);
        if (edge.label) {
          edge.label.setAttribute("x", f(geometry.label.x));
          edge.label.setAttribute("y", f(geometry.label.y - 7));
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

    function positionPackets() {
      for (const packet of packets) {
        const length = packet.edge.path.getTotalLength();
        if (!length) continue;
        const point = packet.edge.path.getPointAtLength(
          packetProgress(packet) * length,
        );
        packet.circle.setAttribute("cx", f(point.x));
        packet.circle.setAttribute("cy", f(point.y));
      }
    }

    function packetRunning() {
      return (
        !state.userPaused && !state.autoPaused && packets.length > 0
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
      for (const packet of packets) {
        packet.phase = (packet.phase + delta) % 1;
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
    }

    function updateDirectionLabel() {
      if (!directionButton) return;
      directionButton.textContent = DIRECTION_LABELS[state.direction];
      directionButton.setAttribute(
        "aria-label",
        `Flow direction: ${DIRECTION_NAMES[state.direction]}`,
      );
    }

    function edgeBaseDuration(edge) {
      if (edge.classList.contains("dg-edge--draw")) return drawMs;
      if (edge.classList.contains("dg-edge--pulse")) return pulseMs;
      return 0;
    }

    function updateAnimations() {
      const running = !state.userPaused && !state.autoPaused;
      animatedEdges.forEach((edge) => {
        if (edge.classList.contains("dg-edge--flow")) {
          edge.style.animation = "";
          return;
        }
        const baseDuration = edgeBaseDuration(edge);
        if (!baseDuration) return;
        edge.style.animationDuration = `${baseDuration / state.speed}ms`;
        edge.style.animationPlayState = running ? "running" : "paused";
      });
      syncPackets();
      updatePlayButton();
    }

    function togglePlay() {
      state.userPaused = !state.userPaused;
      updateAnimations();
    }

    function resetLayout() {
      nodeDrag = null;
      drag = null;
      pinch = null;
      pointers.clear();
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
      animatedEdges.forEach((edge) => {
        if (!edge.classList.contains("dg-edge--draw")) return;
        edge.style.animation = "none";
        void edge.getBoundingClientRect();
        edge.style.animation = "";
      });
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

      clone.querySelectorAll(".dg-edge--draw").forEach((edge) => {
        edge.style.animation = "none";
        edge.style.strokeDashoffset = "0";
        edge.style.strokeDasharray = "1";
      });
      clone.querySelectorAll(".dg-edge--pulse").forEach((edge) => {
        edge.style.animation = "none";
        edge.style.strokeOpacity = "1";
      });
      clone.querySelectorAll(".dg-packet").forEach((packet) => {
        packet.style.opacity = "1";
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
      };
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
          const footer = exported.caption ? 58 : 0;
          const canvas = document.createElement("canvas");
          canvas.width = Math.round(base.w * scale);
          canvas.height = Math.round((base.h + footer) * scale);
          const context = canvas.getContext("2d");
          context.fillStyle = exported.background;
          context.fillRect(0, 0, canvas.width, canvas.height);
          context.drawImage(image, 0, 0, canvas.width, Math.round(base.h * scale));
          URL.revokeObjectURL(url);

          if (exported.caption) {
            const muted = window
              .getComputedStyle(document.documentElement)
              .getPropertyValue("--ink-muted")
              .trim() || "#7f9182";
            context.fillStyle = muted;
            context.font = `${Math.round(10 * scale)}px ui-monospace, SFMono-Regular, Menlo, monospace`;
            context.fillText(
              exported.caption,
              Math.round(20 * scale),
              Math.round((base.h + 28) * scale),
              Math.round((base.w - 40) * scale),
            );
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
      nodeDrag = {
        id: event.pointerId,
        node,
        startX: event.clientX,
        startY: event.clientY,
        startTx: node.tx,
        startTy: node.ty,
      };
      node.element.classList.add("is-moving");
      return true;
    }

    // Animation controls
    playButton?.addEventListener("click", togglePlay);

    speedButton?.addEventListener("click", () => {
      state.speedIndex = (state.speedIndex + 1) % SPEEDS.length;
      state.speed = SPEEDS[state.speedIndex];
      updateSpeedLabel();
      updateAnimations();
    });

    directionButton?.addEventListener("click", () => {
      const next =
        (DIRECTIONS.indexOf(state.direction) + 1) % DIRECTIONS.length;
      state.direction = DIRECTIONS[next];
      updateDirectionLabel();
      updateAnimations();
      positionPackets();
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
        const nodeElement = closestNode(event.target);
        if (nodeElement && (!isMouse || event.button === 0)) {
          if (startNodeDrag(nodeElement, event)) {
            event.preventDefault();
            return;
          }
        }
        startPan(event);
      } else {
        if (nodeDrag) {
          nodeDrag.node.element.classList.remove("is-moving");
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
        const dx = (event.clientX - nodeDrag.startX) / state.scale;
        const dy = (event.clientY - nodeDrag.startY) / state.scale;
        nodeDrag.node.tx = nodeDrag.startTx + dx;
        nodeDrag.node.ty = nodeDrag.startTy + dy;
        nodeDrag.node.element.setAttribute(
          "transform",
          `translate(${f(nodeDrag.node.tx)} ${f(nodeDrag.node.ty)})`,
        );
        updateEdges();
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
        nodeDrag.node.element.classList.remove("is-moving");
        nodeDrag = null;
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
    updateHintText();
    MOBILE.addEventListener("change", updateHintText);

    window.requestAnimationFrame(() => {
      initialView();
      positionPackets();
      updateAnimations();
      toolbar.hidden = false;
      if (hint) hint.hidden = false;
      figure.setAttribute("data-dg-ready", "");
    });
  }
})();
