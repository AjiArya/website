(() => {
  const canvas = document.querySelector("[data-sky]");
  const ctx = canvas?.getContext?.("2d");
  if (!ctx) return;

  const random = Math.random;
  const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const dimmed = canvas.hasAttribute("data-dim");

  const config = dimmed
    ? { maxStars: 500, areaPerStar: 2600, alpha: 0.6 }
    : { maxStars: 900, areaPerStar: 1600, alpha: 1 };

  const nextShotDelay = () => {
    if (dimmed) return 8 + random() * 10;
    return random() < 0.18 ? 0.3 + random() * 0.5 : 3.5 + random() * 6;
  };

  const sprite = (stops) => {
    const element = document.createElement("canvas");
    element.width = element.height = 32;
    const spriteCtx = element.getContext("2d");
    const gradient = spriteCtx.createRadialGradient(16, 16, 0, 16, 16, 16);
    stops.forEach(([offset, color]) => gradient.addColorStop(offset, color));
    spriteCtx.fillStyle = gradient;
    spriteCtx.fillRect(0, 0, 32, 32);
    return element;
  };

  const sprites = [
    sprite([[0, "rgba(240,255,240,1)"], [0.18, "rgba(134,239,172,.9)"], [0.45, "rgba(34,197,94,.25)"], [1, "rgba(28,185,0,0)"]]),
    sprite([[0, "rgba(187,247,208,1)"], [0.2, "rgba(74,222,128,.8)"], [0.5, "rgba(28,185,0,.2)"], [1, "rgba(28,185,0,0)"]]),
    sprite([[0, "rgba(74,222,128,.9)"], [0.25, "rgba(28,185,0,.6)"], [0.6, "rgba(21,128,61,.15)"], [1, "rgba(21,128,61,0)"]]),
  ];

  const mouse = { x: 0, y: 0, targetX: 0, targetY: 0 };
  let width = 0;
  let height = 0;
  let stars = [];
  let shots = [];
  let frame = 0;
  let last = performance.now();
  let time = 0;
  let nextShot = 2.5;

  const drawShot = (shot) => {
    const fade = Math.sin((Math.PI * shot.life) / shot.max);
    const speed = Math.hypot(shot.vx, shot.vy);
    const tailX = shot.x - (shot.vx / speed) * shot.length;
    const tailY = shot.y - (shot.vy / speed) * shot.length;

    ctx.globalAlpha = 1;
    ctx.lineCap = "round";

    let gradient = ctx.createLinearGradient(tailX, tailY, shot.x, shot.y);
    gradient.addColorStop(0, "rgba(28,185,0,0)");
    gradient.addColorStop(1, `rgba(74,222,128,${0.22 * fade})`);
    ctx.strokeStyle = gradient;
    ctx.lineWidth = 6;
    ctx.beginPath();
    ctx.moveTo(tailX, tailY);
    ctx.lineTo(shot.x, shot.y);
    ctx.stroke();

    gradient = ctx.createLinearGradient(tailX, tailY, shot.x, shot.y);
    gradient.addColorStop(0, "rgba(28,185,0,0)");
    gradient.addColorStop(0.7, `rgba(74,222,128,${0.45 * fade})`);
    gradient.addColorStop(1, `rgba(230,255,230,${fade})`);
    ctx.strokeStyle = gradient;
    ctx.lineWidth = 1.6;
    ctx.beginPath();
    ctx.moveTo(tailX, tailY);
    ctx.lineTo(shot.x, shot.y);
    ctx.stroke();

    ctx.globalAlpha = fade;
    ctx.drawImage(sprites[0], shot.x - 9, shot.y - 9, 18, 18);
  };

  const draw = () => {
    ctx.clearRect(0, 0, width, height);
    ctx.globalCompositeOperation = "lighter";

    const offsetX = (mouse.x - width / 2) * -0.014;
    const offsetY = (mouse.y - height / 2) * -0.014;

    for (const star of stars) {
      const twinkle = 0.5 + 0.5 * Math.sin(time * star.speed + star.phase);
      const flare = twinkle > 0.97 && star.tier === 0 ? 1.6 : 1;
      const size = star.radius * (5 + 3 * twinkle) * flare;
      ctx.globalAlpha = star.alpha * (0.3 + 0.7 * twinkle) * config.alpha;
      ctx.drawImage(
        sprites[star.tier],
        star.x + offsetX * star.depth - size / 2,
        star.y + offsetY * star.depth - size / 2,
        size,
        size,
      );
    }

    shots.forEach(drawShot);

    ctx.globalAlpha = 1;
    ctx.globalCompositeOperation = "source-over";
  };

  const resize = () => {
    const dpr = Math.min(window.devicePixelRatio || 1, 1.5);
    width = canvas.clientWidth;
    height = canvas.clientHeight;
    if (!width || !height) return;

    canvas.width = Math.round(width * dpr);
    canvas.height = Math.round(height * dpr);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    mouse.x = mouse.targetX = width / 2;
    mouse.y = mouse.targetY = height / 2;

    const count = Math.round(Math.min(config.maxStars, (width * height) / config.areaPerStar));
    stars = Array.from({ length: count }, () => {
      const tier = random() < 0.1 ? 0 : random() < 0.55 ? 1 : 2;
      return {
        x: random() * width,
        y: random() * height,
        depth: random(),
        radius: 0.35 + Math.pow(random(), 3) * 1.5 + (tier === 0 ? 0.4 : 0),
        alpha: 0.25 + random() * 0.6,
        speed: 0.3 + random() * 1.8,
        phase: random() * 6.283,
        tier,
      };
    });

    if (reduceMotion) draw();
  };

  const shoot = (x, y) => {
    if (reduceMotion) return;
    const direction = random() < 0.5 ? 1 : -1;
    const angle = ((16 + random() * 24) * Math.PI) / 180;
    const speed = 13 + random() * 9;
    shots.push({
      x: x ?? width * (0.1 + random() * 0.8),
      y: y ?? height * random() * 0.45,
      vx: Math.cos(angle) * speed * direction,
      vy: Math.sin(angle) * speed,
      length: 110 + random() * 170,
      life: 0,
      max: 40 + random() * 30,
    });
  };

  const step = (delta) => {
    time += delta / 60;
    nextShot -= delta / 60;
    if (nextShot <= 0) {
      shoot();
      nextShot = nextShotDelay();
    }

    for (const shot of shots) {
      shot.x += shot.vx * delta;
      shot.y += shot.vy * delta;
      shot.life += delta;
    }
    shots = shots.filter((shot) => shot.life < shot.max);

    mouse.x += (mouse.targetX - mouse.x) * 0.04;
    mouse.y += (mouse.targetY - mouse.y) * 0.04;
  };

  const loop = (now) => {
    frame = requestAnimationFrame(loop);
    if (document.hidden) {
      last = now;
      return;
    }
    const delta = Math.min(3, (now - last) / 16.67);
    last = now;
    step(delta);
    draw();
  };

  resize();
  if ("ResizeObserver" in window) {
    new ResizeObserver(resize).observe(canvas);
  }

  if (reduceMotion) return;
  frame = requestAnimationFrame(loop);

  document.addEventListener("mousemove", (event) => {
    const rect = canvas.getBoundingClientRect();
    mouse.targetX = event.clientX - rect.left;
    mouse.targetY = event.clientY - rect.top;
  });

  document.documentElement.addEventListener("mouseleave", () => {
    mouse.targetX = width / 2;
    mouse.targetY = height / 2;
  });

  // Clicks on empty space fire a shot. Text and controls keep their own behaviour.
  const notEmptySpace =
    "a, button, input, label, summary, select, textarea, p, li, h1, h2, h3, h4, pre, code, blockquote, table, dl, img, .diagram, .terminal";
  document.addEventListener("mousedown", (event) => {
    if (event.target.closest?.(notEmptySpace)) return;
    const rect = canvas.getBoundingClientRect();
    shoot(event.clientX - rect.left, event.clientY - rect.top);
  });

  window.addEventListener("pagehide", () => cancelAnimationFrame(frame));
})();
