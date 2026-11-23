const detectMac = () => {
	if (typeof navigator === "undefined") return false;
	const platform =
		(navigator as any).userAgentData?.platform ?? navigator.platform ?? "";
	return /mac/i.test(platform) || /mac/i.test(navigator.userAgent);
};

export const isMac = detectMac();

/** Label of the primary modifier key, for shortcut hints. */
export const modKey = isMac ? "Cmd" : "Ctrl";
