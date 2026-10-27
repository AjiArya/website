(() => {
  const menuToggle = document.querySelector("[data-menu-toggle]");
  const siteNav = document.querySelector("[data-site-nav]");

  if (menuToggle && siteNav) {
    const setMenuOpen = (open) => {
      siteNav.classList.toggle("is-open", open);
      menuToggle.setAttribute("aria-expanded", String(open));
      menuToggle.setAttribute("aria-label", open ? "Close menu" : "Open menu");
    };

    menuToggle.addEventListener("click", (event) => {
      event.stopPropagation();
      setMenuOpen(!siteNav.classList.contains("is-open"));
    });

    siteNav.querySelectorAll("a").forEach((link) => {
      link.addEventListener("click", () => setMenuOpen(false));
    });

    document.addEventListener("click", (event) => {
      if (!siteNav.contains(event.target) && !menuToggle.contains(event.target)) {
        setMenuOpen(false);
      }
    });

    document.addEventListener("keydown", (event) => {
      if (event.key === "Escape" && siteNav.classList.contains("is-open")) {
        setMenuOpen(false);
        menuToggle.focus();
      }
    });

    const desktop = window.matchMedia("(min-width: 901px)");
    desktop.addEventListener("change", (event) => {
      if (event.matches) setMenuOpen(false);
    });
  }

  document.querySelectorAll(".prose table").forEach((table) => {
    const wrapper = document.createElement("div");
    wrapper.className = "table-scroll";
    table.parentNode.insertBefore(wrapper, table);
    wrapper.appendChild(table);
  });

  document.querySelectorAll(".prose pre").forEach((pre) => {
    const code = pre.querySelector("code");
    if (!code) return;

    const button = document.createElement("button");
    button.type = "button";
    button.className = "code-copy";
    button.textContent = "Copy";
    button.setAttribute("aria-label", "Copy code to clipboard");
    pre.appendChild(button);

    button.addEventListener("click", async () => {
      try {
        await navigator.clipboard.writeText(code.innerText);
        button.textContent = "Copied";
        button.dataset.copied = "true";
        window.setTimeout(() => {
          button.textContent = "Copy";
          button.dataset.copied = "false";
        }, 1600);
      } catch (_) {
        button.textContent = "Select + copy";
      }
    });
  });
})();
