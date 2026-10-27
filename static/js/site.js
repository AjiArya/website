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
      type: "nav",
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

    const commands = [
      {
        id: "search",
        title: "/search",
        aliases: ["find"],
        summary: "Search pages, posts, tags, and topics.",
        fill: "/search ",
        meta: "/find · <query>",
      },
      {
        id: "home",
        title: "/home",
        summary: "Jump to the home page.",
        fill: "/home",
        meta: "page",
      },
      {
        id: "share",
        title: "/share",
        aliases: [],
        summary: "Share this page, or pick a platform.",
        fill: "/share ",
        meta: "<social>",
      },
      {
        id: "help",
        title: "/help",
        summary: "Show commands and keyboard shortcuts.",
        fill: "/help",
        meta: "reference",
      },
    ];
    const commandById = new Map(
      commands.map((command) => [command.id, command]),
    );
    const commandLookup = new Map();
    commands.forEach((command) => {
      commandLookup.set(command.id, command.id);
      (command.aliases || []).forEach((alias) =>
        commandLookup.set(alias, command.id),
      );
    });

    const socials = [
      {
        id: "x",
        title: "X",
        aliases: ["twitter"],
        summary: "Post this page on X.",
        meta: "twitter.com",
      },
      {
        id: "linkedin",
        title: "LinkedIn",
        aliases: [],
        summary: "Share this page on LinkedIn.",
        meta: "linkedin.com",
      },
      {
        id: "facebook",
        title: "Facebook",
        aliases: [],
        summary: "Share this page on Facebook.",
        meta: "facebook.com",
      },
      {
        id: "reddit",
        title: "Reddit",
        aliases: [],
        summary: "Submit this page to Reddit.",
        meta: "reddit.com",
      },
      {
        id: "email",
        title: "Email",
        aliases: ["mail"],
        summary: "Send this page by email.",
        meta: "mailto",
      },
      {
        id: "copy",
        title: "Copy link",
        aliases: ["clipboard", "url"],
        summary: "Copy this page URL to the clipboard.",
        meta: "clipboard",
      },
    ];

    const terminalInput = terminal.querySelector("[data-terminal-input]");
    const terminalForm = terminal.querySelector("[data-terminal-form]");
    const results = terminal.querySelector("[data-terminal-results]");
    const empty = terminal.querySelector("[data-terminal-empty]");
    const helpPanel = terminal.querySelector("[data-terminal-help]");
    const completion = terminal.querySelector(
      "[data-terminal-completion]",
    );
    const status = terminal.querySelector("[data-terminal-status]");
    const count = terminal.querySelector("[data-terminal-count]");
    const triggerButtons = Array.from(
      document.querySelectorAll("[data-terminal-open]"),
    );
    const closeButtons = Array.from(
      terminal.querySelectorAll("[data-terminal-close]"),
    );

    const DEFAULT_STATUS = "/search · /home · /share · /help";
    let currentItems = [];
    let selectedIndex = 0;
    let lastFocus = null;
    let statusTimer = null;

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

    const setStatus = (message) => {
      window.clearTimeout(statusTimer);
      status.textContent = message;
    };

    const flashStatus = (message) => {
      setStatus(message);
      statusTimer = window.setTimeout(() => {
        status.textContent = DEFAULT_STATUS;
      }, 2200);
    };

    const matchCommands = (prefix) => {
      const query = prefix.toLowerCase();
      return commands.filter(
        (command) =>
          command.title.slice(1).startsWith(query) ||
          (command.aliases || []).some((alias) =>
            alias.startsWith(query),
          ),
      );
    };

    const matchSocials = (value) => {
      const query = value.toLowerCase();
      if (!query) return socials;
      return socials.filter(
        (social) =>
          social.id.startsWith(query) ||
          social.title.toLowerCase().startsWith(query) ||
          social.aliases.some((alias) => alias.startsWith(query)) ||
          social.title.toLowerCase().includes(query) ||
          social.aliases.some((alias) => alias.includes(query)),
      );
    };

    const parseQuery = (value) => {
      const raw = value.trimStart();
      if (!raw.startsWith("/")) {
        return { mode: "search", query: raw.trim() };
      }

      const body = raw.slice(1);
      const tokenMatch = body.match(/^(\S*)(?:\s+([\s\S]*))?$/);
      const token = (tokenMatch?.[1] || "").toLowerCase();
      const rest = (tokenMatch?.[2] || "").trim();

      if (!token) return { mode: "commands", prefix: "" };

      const commandId = commandLookup.get(token);
      if (commandId === "search") {
        return { mode: "search", query: rest, slashSearch: true };
      }
      if (commandId === "share") {
        return { mode: "share", query: rest };
      }
      if (commandId === "help") return { mode: "help" };
      if (commandId === "home") {
        return { mode: "command", command: "home" };
      }
      if (matchCommands(token).length) {
        return { mode: "commands", prefix: token };
      }

      return { mode: "search", query: body.trim(), unknown: token };
    };

    const searchItems = (value) => {
      const query = value.trim().toLowerCase();
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
          (a, b) => a.score - b.score || a.item.order - b.item.order,
        )
        .slice(0, 20)
        .map((entry) => entry.item);
    };

    const commandItem = (command, order) => ({
      type: "command",
      kind: "command",
      title: command.title,
      summary: command.summary,
      meta: command.meta,
      fill: command.fill,
      command: command.id,
      order,
    });

    const socialItem = (social, order) => ({
      type: "share",
      kind: "share",
      title: social.title,
      summary: social.summary,
      meta: social.meta,
      social: social.id,
      order,
    });

    const updateCompletion = () => {
      if (!completion) return;
      const value = terminalInput.value.trimEnd();
      const selected = currentItems[selectedIndex];
      let suggestion = "";

      if (selected?.type === "command") {
        suggestion = selected.fill.trim();
      } else if (selected?.type === "share") {
        suggestion = `/share ${selected.social}`;
      }

      if (
        suggestion &&
        suggestion !== value &&
        suggestion.startsWith(value)
      ) {
        completion.textContent = `⇥ ${suggestion}`;
        completion.hidden = false;
      } else {
        completion.textContent = "";
        completion.hidden = true;
      }
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
      updateCompletion();
    };

    const renderOptions = () => {
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
    };

    const showHelp = () => {
      helpPanel.hidden = false;
      results.hidden = true;
      empty.hidden = true;
      completion.hidden = true;
      currentItems = [];
      selectedIndex = -1;
      count.textContent = "";
      terminalInput.value = "";
      terminalInput.removeAttribute("aria-activedescendant");
      terminalInput.setAttribute("aria-expanded", "false");
      setStatus("Commands & shortcuts · type to search");
      terminalInput.focus();
    };

    const renderResults = (value) => {
      const parsed = parseQuery(value);

      if (parsed.mode === "help") {
        showHelp();
        return;
      }

      helpPanel.hidden = true;
      results.hidden = false;
      terminalInput.setAttribute("aria-expanded", "true");

      let nextItems = [];
      let emptyMessage = "No matching route.";
      let statusMessage = DEFAULT_STATUS;

      if (parsed.mode === "commands") {
        nextItems = matchCommands(parsed.prefix).map(commandItem);
        statusMessage = "Tab autocomplete · Enter run";
        emptyMessage = "No matching command. Type /help.";
      } else if (parsed.mode === "command") {
        nextItems = [commandItem(commandById.get(parsed.command), 0)];
        statusMessage = "Press Enter to run";
      } else if (parsed.mode === "share") {
        nextItems = matchSocials(parsed.query).map(socialItem);
        statusMessage = parsed.query
          ? "Enter to share"
          : "Choose a platform";
        emptyMessage = `No platform matches “${parsed.query}”. Try /share.`;
      } else {
        nextItems = searchItems(parsed.query);
        if (parsed.unknown) {
          statusMessage = nextItems.length
            ? `Searching for “${parsed.query}”`
            : `Unknown command /${parsed.unknown}`;
          emptyMessage = "No matching route or command. Type /help.";
        }
      }

      currentItems = nextItems;
      renderOptions();

      if (parsed.mode === "share" && !parsed.query) {
        selectedIndex = -1;
      }
      if (selectedIndex >= currentItems.length) {
        selectedIndex = Math.max(0, currentItems.length - 1);
      }

      empty.textContent = emptyMessage;
      empty.hidden = currentItems.length > 0;
      const noun =
        parsed.mode === "commands" || parsed.mode === "command"
          ? "command"
          : parsed.mode === "share"
            ? "platform"
            : "route";
      count.textContent = currentItems.length
        ? `${currentItems.length} ${noun}${
            currentItems.length === 1 ? "" : "s"
          }`
        : "";
      setStatus(statusMessage);
      updateSelection();
    };

    const formatShareUrl = (social, url, title) => {
      switch (social) {
        case "x":
          return `https://twitter.com/intent/tweet?text=${encodeURIComponent(
            title,
          )}&url=${encodeURIComponent(url)}`;
        case "linkedin":
          return `https://www.linkedin.com/sharing/share-offsite/?url=${encodeURIComponent(
            url,
          )}`;
        case "facebook":
          return `https://www.facebook.com/sharer/sharer.php?u=${encodeURIComponent(
            url,
          )}`;
        case "reddit":
          return `https://www.reddit.com/submit?url=${encodeURIComponent(
            url,
          )}&title=${encodeURIComponent(title)}`;
        case "email":
          return `mailto:?subject=${encodeURIComponent(
            title,
          )}&body=${encodeURIComponent(url)}`;
        default:
          return "";
      }
    };

    const pageUrl = () =>
      document.querySelector('link[rel="canonical"]')?.href ||
      window.location.href.split("#")[0];

    const pageTitle = () =>
      document.querySelector('meta[property="og:title"]')?.content ||
      document.title;

    const copyPageLink = async () => {
      const url = pageUrl();
      try {
        await navigator.clipboard.writeText(url);
        flashStatus("Link copied to clipboard");
      } catch (_) {
        const textarea = document.createElement("textarea");
        textarea.value = url;
        textarea.setAttribute("readonly", "");
        textarea.style.position = "fixed";
        textarea.style.opacity = "0";
        document.body.appendChild(textarea);
        textarea.select();
        let copied = false;
        try {
          copied = document.execCommand("copy");
        } catch (_) {
          copied = false;
        }
        textarea.remove();
        flashStatus(
          copied ? "Link copied to clipboard" : "Could not copy link",
        );
      }
      terminalInput.focus();
      const end = terminalInput.value.length;
      terminalInput.setSelectionRange(end, end);
    };

    const sharePage = (social) => {
      if (social === "copy") {
        copyPageLink();
        return;
      }
      const url = pageUrl();
      const title = pageTitle();
      const target = formatShareUrl(social, url, title);
      if (!target) return;
      closeTerminal(false);
      if (social === "email") {
        window.location.href = target;
      } else {
        window.open(target, "_blank", "noopener");
      }
    };

    const goHome = () => {
      const home = items.find(
        (item) => item.kind === "page" && item.path === "/",
      );
      const homeUrl = home?.url || "/";
      const normalizedPath = (value) =>
        new URL(value, window.location.href).pathname.replace(/\/+$/, "");
      closeTerminal(false);
      if (normalizedPath(homeUrl) === normalizedPath(window.location.href)) {
        window.scrollTo({
          top: 0,
          behavior: window.matchMedia("(prefers-reduced-motion: reduce)")
            .matches
            ? "auto"
            : "smooth",
        });
      } else {
        window.location.assign(homeUrl);
      }
    };

    const fillInput = (value) => {
      terminalInput.value = value;
      selectedIndex = 0;
      terminalInput.focus();
      const end = terminalInput.value.length;
      terminalInput.setSelectionRange(end, end);
      renderResults(value);
    };

    function activateItem(item) {
      if (!item) return;
      if (item.type === "command") {
        if (item.command === "home") {
          goHome();
        } else {
          fillInput(item.fill);
        }
        return;
      }
      if (item.type === "share") {
        sharePage(item.social);
        return;
      }
      if (!item.url) return;
      closeTerminal(false);
      if (item.newTab) {
        window.open(item.url, "_blank", "noopener");
      } else {
        window.location.assign(item.url);
      }
    }

    const completeSelection = () => {
      const item = currentItems[selectedIndex];
      if (!item) return;
      if (item.type === "command") {
        fillInput(item.fill);
      } else if (item.type === "share") {
        fillInput(`/share ${item.social}`);
      }
    };

    function openTerminal(initialValue = "") {
      if (!terminal.hidden) {
        terminalInput.focus();
        return;
      }
      lastFocus = document.activeElement;
      terminal.hidden = false;
      document.body.classList.add("terminal-open");
      helpPanel.hidden = true;
      results.hidden = false;
      terminalInput.value = initialValue;
      terminalInput.setAttribute("aria-expanded", "true");
      selectedIndex = 0;
      renderResults(initialValue);
      window.requestAnimationFrame(() => {
        terminalInput.focus();
        const end = terminalInput.value.length;
        terminalInput.setSelectionRange(end, end);
      });
    }

    function closeTerminal(restoreFocus = true) {
      if (terminal.hidden) return;
      if (terminal.contains(document.activeElement)) {
        document.activeElement.blur();
      }
      terminal.hidden = true;
      document.body.classList.remove("terminal-open");
      terminalInput.setAttribute("aria-expanded", "false");
      if (restoreFocus && lastFocus && document.contains(lastFocus)) {
        lastFocus.focus();
      }
    }

    triggerButtons.forEach((button) => {
      button.addEventListener("click", () => openTerminal());
    });
    closeButtons.forEach((button) => {
      button.addEventListener("click", () => closeTerminal());
    });

    terminalForm?.addEventListener("submit", (event) => {
      event.preventDefault();
      const item = currentItems[selectedIndex];
      if (item) {
        activateItem(item);
      } else if (parseQuery(terminalInput.value).mode === "share") {
        flashStatus("Choose a platform first");
      }
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
              selectedIndex < 0
                ? 0
                : (selectedIndex + 1) % currentItems.length;
            updateSelection();
          }
          event.preventDefault();
          break;
        case "ArrowUp":
          if (currentItems.length) {
            selectedIndex =
              selectedIndex < 0
                ? currentItems.length - 1
                : (selectedIndex - 1 + currentItems.length) %
                  currentItems.length;
            updateSelection();
          }
          event.preventDefault();
          break;
        case "Enter":
          if (currentItems[selectedIndex]) {
            event.preventDefault();
            activateItem(currentItems[selectedIndex]);
          } else if (parseQuery(terminalInput.value).mode === "share") {
            event.preventDefault();
            flashStatus("Choose a platform first");
          }
          break;
        case "Escape":
          event.preventDefault();
          closeTerminal();
          break;
        case "Tab":
          event.preventDefault();
          completeSelection();
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
        (!isEditable(event.target) || terminal.contains(event.target))
      ) {
        event.preventDefault();
        openTerminal("/");
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
