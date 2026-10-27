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

  const terminal = document.querySelector("[data-terminal]");
  if (terminal) {
    const indexLinks = Array.from(
      document.querySelectorAll("#terminal-index [data-nav-item]"),
    );
    const items = indexLinks.map((link, order) => ({
      title: link.dataset.title || "Untitled",
      summary: link.dataset.summary || "",
      keywords: (link.dataset.keywords || "").toLowerCase(),
      kind: link.dataset.kind || "page",
      path: link.dataset.path || "",
      meta: link.dataset.meta || "",
      url: link.getAttribute("href") || "",
      newTab: link.dataset.newTab === "true",
      order,
    }));

    const terminalInput = terminal.querySelector("[data-terminal-input]");
    const terminalForm = terminal.querySelector("[data-terminal-form]");
    const results = terminal.querySelector("[data-terminal-results]");
    const empty = terminal.querySelector("[data-terminal-empty]");
    const count = terminal.querySelector("[data-terminal-count]");
    const triggerButtons = Array.from(
      document.querySelectorAll("[data-terminal-open]"),
    );
    const closeButtons = Array.from(
      terminal.querySelectorAll("[data-terminal-close]"),
    );

    let currentItems = [];
    let selectedIndex = 0;
    let lastFocus = null;

    const isEditable = (element) => {
      if (!element || !element.tagName) return false;
      const tag = element.tagName;
      return (
        tag === "INPUT" ||
        tag === "TEXTAREA" ||
        tag === "SELECT" ||
        element.isContentEditable
      );
    };

    const normalizeQuery = (value) => {
      let query = value.trim().toLowerCase();
      while (query.startsWith("/") || query.startsWith(">")) {
        query = query.slice(1).trimStart();
      }
      return query;
    };

    const searchItems = (value) => {
      const query = normalizeQuery(value);
      if (!query) return items;

      return items
        .map((item) => {
          const title = item.title.toLowerCase();
          let score = Infinity;
          if (title.startsWith(query)) score = 0;
          else if (title.includes(query)) score = 1;
          else if (item.keywords.includes(query)) score = 2;
          else if (item.summary.toLowerCase().includes(query)) score = 3;
          else if (item.path.toLowerCase().includes(query)) score = 4;
          return { item, score };
        })
        .filter((entry) => Number.isFinite(entry.score))
        .sort(
          (a, b) =>
            a.score - b.score || a.item.order - b.item.order,
        )
        .slice(0, 20)
        .map((entry) => entry.item);
    };

    const updateSelection = () => {
      const options = Array.from(
        results.querySelectorAll('[role="option"]'),
      );
      options.forEach((option, index) => {
        const active = index === selectedIndex;
        option.classList.toggle("is-selected", active);
        option.setAttribute("aria-selected", String(active));
      });

      const active = options[selectedIndex];
      if (active) {
        terminalInput.setAttribute("aria-activedescendant", active.id);
        active.scrollIntoView({ block: "nearest" });
      } else {
        terminalInput.removeAttribute("aria-activedescendant");
      }
    };

    const renderResults = (value) => {
      currentItems = searchItems(value);
      results.replaceChildren();

      currentItems.forEach((item, index) => {
        const option = document.createElement("button");
        option.type = "button";
        option.id = `terminal-option-${index}`;
        option.className = "terminal__result";
        option.setAttribute("role", "option");
        option.setAttribute("aria-selected", "false");
        option.tabIndex = -1;

        const kind = document.createElement("span");
        kind.className = "terminal__result-kind";
        kind.textContent = item.kind;

        const content = document.createElement("span");
        content.className = "terminal__result-content";
        const title = document.createElement("span");
        title.className = "terminal__result-title";
        title.textContent = item.title;
        content.appendChild(title);
        if (item.summary) {
          const summary = document.createElement("span");
          summary.className = "terminal__result-summary";
          summary.textContent = item.summary;
          content.appendChild(summary);
        }

        const meta = document.createElement("span");
        meta.className = "terminal__result-meta";
        meta.textContent = item.meta || item.path;

        option.append(kind, content, meta);
        option.addEventListener("click", () => activateItem(item));
        results.appendChild(option);
      });

      if (selectedIndex >= currentItems.length) {
        selectedIndex = Math.max(0, currentItems.length - 1);
      }
      empty.hidden = currentItems.length > 0;
      count.textContent = `${currentItems.length} route${
        currentItems.length === 1 ? "" : "s"
      }`;
      updateSelection();
    };

    function activateItem(item) {
      if (!item || !item.url) return;
      closeTerminal(false);
      if (item.newTab) {
        window.open(item.url, "_blank", "noopener");
      } else {
        window.location.assign(item.url);
      }
    }

    function openTerminal() {
      if (!terminal.hidden) {
        terminalInput.focus();
        return;
      }
      lastFocus = document.activeElement;
      terminal.hidden = false;
      document.body.classList.add("terminal-open");
      terminalInput.value = "";
      terminalInput.setAttribute("aria-expanded", "true");
      selectedIndex = 0;
      renderResults("");
      window.requestAnimationFrame(() => {
        terminalInput.focus();
      });
    }

    function closeTerminal(restoreFocus = true) {
      if (terminal.hidden) return;
      terminal.hidden = true;
      document.body.classList.remove("terminal-open");
      terminalInput.setAttribute("aria-expanded", "false");
      if (restoreFocus && lastFocus && document.contains(lastFocus)) {
        lastFocus.focus();
      }
    }

    triggerButtons.forEach((button) => {
      button.addEventListener("click", openTerminal);
    });
    closeButtons.forEach((button) => {
      button.addEventListener("click", () => closeTerminal());
    });

    terminalForm?.addEventListener("submit", (event) => {
      event.preventDefault();
      activateItem(currentItems[selectedIndex]);
    });

    terminalInput?.addEventListener("input", () => {
      selectedIndex = 0;
      renderResults(terminalInput.value);
    });

    terminalInput?.addEventListener("keydown", (event) => {
      switch (event.key) {
        case "ArrowDown":
          if (currentItems.length) {
            selectedIndex =
              (selectedIndex + 1) % currentItems.length;
            updateSelection();
          }
          event.preventDefault();
          break;
        case "ArrowUp":
          if (currentItems.length) {
            selectedIndex =
              (selectedIndex - 1 + currentItems.length) %
              currentItems.length;
            updateSelection();
          }
          event.preventDefault();
          break;
        case "Enter":
          if (currentItems[selectedIndex]) {
            event.preventDefault();
            activateItem(currentItems[selectedIndex]);
          }
          break;
        case "Escape":
          event.preventDefault();
          closeTerminal();
          break;
        case "Tab":
          event.preventDefault();
          break;
        default:
          break;
      }
    });

    document.addEventListener("keydown", (event) => {
      if (event.defaultPrevented) return;

      if (
        terminal.hidden &&
        event.key === "/" &&
        !event.metaKey &&
        !event.ctrlKey &&
        !event.altKey &&
        !isEditable(event.target)
      ) {
        event.preventDefault();
        openTerminal();
        return;
      }

      if (!terminal.hidden && event.key === "Escape") {
        event.preventDefault();
        closeTerminal();
      }
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
