(function () {
    document.addEventListener("DOMContentLoaded", async () => {
        const rightButtons = document.querySelector(".right-buttons");
        if (!rightButtons) return;

        // Detect Base URL and Current Version
        const path = window.location.pathname;
        const segments = path.split('/').filter(Boolean);

        // Find the index of the version segment (main, latest, or vX.Y.Z)
        const versionIndex = segments.findIndex(s => s === 'main' || s === 'latest' || /^v\d/.test(s));

        if (versionIndex === -1) {
            console.warn("waypane-docs: Could not detect version in path", path);
            return;
        }

        const currentVersion = segments[versionIndex];
        const baseUrl = '/' + segments.slice(0, versionIndex).join('/') + (versionIndex > 0 ? '/' : '');

        // Create UI
        const select = document.createElement("select");
        select.id = "version-picker";
        Object.assign(select.style, {
            margin: "0 10px",
            padding: "2px 8px",
            borderRadius: "4px",
            fontSize: "0.8em",
            background: "var(--sidebar-bg)",
            color: "var(--sidebar-fg)",
            border: "1px solid var(--searchbar-border-color)",
            cursor: "pointer",
            verticalAlign: "middle"
        });

        const addOption = (val, text) => {
            const opt = document.createElement("option");
            opt.value = val;
            opt.textContent = text;
            if (val === currentVersion) opt.selected = true;
            select.appendChild(opt);
        };

        // Fetch manifest
        try {
            const res = await fetch(`${baseUrl}versions.json`);
            if (!res.ok) throw new Error("Manifest not found");
            const versions = await res.json();

            versions.forEach(v => {
                let label = v;
                if (v === 'main') label = 'main (nightly)';
                if (v === 'latest') label = 'latest (stable)';
                addOption(v, label);
            });
        } catch (e) {
            console.warn("waypane-docs: Using fallback version list", e);
            addOption(currentVersion, currentVersion);
        }

        // Navigation
        select.addEventListener("change", (e) => {
            const targetVersion = e.target.value;
            if (targetVersion === currentVersion) return;

            const newSegments = [...segments];
            newSegments[versionIndex] = targetVersion;

            window.location.href = '/' + newSegments.join('/') + (path.endsWith('/') ? '/' : '');
        });

        const searchBtn = document.getElementById("search-toggle");
        if (searchBtn) {
            rightButtons.insertBefore(select, searchBtn);
        } else {
            rightButtons.prepend(select);
        }
    });
})();
