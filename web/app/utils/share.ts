export type SharedEditorFile = {
	fileName: string;
	content: string;
};

const DEFAULT_SHARED_FILE_NAME = "shared-link.alt";
const DATA_PARAM_KEYS = ["data"];
const DATA64_PARAM_KEYS = ["data64", "base64"];
const FILE_NAME_PARAM_KEYS = ["fileName", "filename", "file"];
const SHARED_EDITOR_PARAM_KEYS = [
	...DATA_PARAM_KEYS,
	...DATA64_PARAM_KEYS,
	...FILE_NAME_PARAM_KEYS,
];

const getSearchParamsCandidates = (): URLSearchParams[] => {
	const candidates = [new URLSearchParams(window.location.search)];
	const queryIndex = window.location.hash.indexOf("?");

	if (queryIndex >= 0) {
		candidates.push(
			new URLSearchParams(window.location.hash.slice(queryIndex)),
		);
	}

	return candidates;
};

const getFirstParamValue = (keys: string[]): string | null => {
	for (const params of getSearchParamsCandidates()) {
		for (const key of keys) {
			const value = params.get(key);
			if (value !== null) {
				return value;
			}
		}
	}

	return null;
};

const decodeBase64Url = (encodedContent: string): string | null => {
	try {
		const normalized = encodedContent.replace(/-/g, "+").replace(/_/g, "/");
		const paddingLength = (4 - (normalized.length % 4)) % 4;
		const padded = normalized + "=".repeat(paddingLength);
		const binary = atob(padded);
		const bytes = Uint8Array.from(binary, (character) =>
			character.charCodeAt(0),
		);

		return new TextDecoder().decode(bytes);
	} catch (error) {
		console.warn(
			"Unable to decode shared editor payload from URL query parameters.",
			error,
		);
		return null;
	}
};

const sanitizeFileName = (fileName: string | null): string => {
	const trimmed = fileName?.trim() || DEFAULT_SHARED_FILE_NAME;
	const baseName =
		trimmed.split(/[\\/]/).filter((segment) => segment.length > 0).pop() ||
		DEFAULT_SHARED_FILE_NAME;
	const safeName = baseName.replace(/[<>:"|?*\u0000-\u001F]/g, "-").trim();

	if (safeName.length === 0) {
		return DEFAULT_SHARED_FILE_NAME;
	}

	return safeName.includes(".") ? safeName : `${safeName}.alt`;
};

export const loadSharedEditorFileFromUrl = (): SharedEditorFile | null => {
	if (typeof window === "undefined") {
		return null;
	}

	const encodedBase64Content = getFirstParamValue(DATA64_PARAM_KEYS);
	const plainContent = getFirstParamValue(DATA_PARAM_KEYS);
	const fileName = sanitizeFileName(getFirstParamValue(FILE_NAME_PARAM_KEYS));

	if (encodedBase64Content !== null) {
		const content = decodeBase64Url(encodedBase64Content);
		return content === null ? null : { fileName, content };
	}

	if (plainContent === null) {
		return null;
	}

	return {
		fileName,
		content: plainContent,
	};
};

const removeSharedEditorParams = (params: URLSearchParams): boolean => {
	let changed = false;

	for (const key of SHARED_EDITOR_PARAM_KEYS) {
		if (params.has(key)) {
			params.delete(key);
			changed = true;
		}
	}

	return changed;
};

export const clearSharedEditorFileFromUrl = () => {
	if (typeof window === "undefined") {
		return;
	}

	const url = new URL(window.location.href);
	let changed = removeSharedEditorParams(url.searchParams);
	const queryIndex = url.hash.indexOf("?");

	if (queryIndex >= 0) {
		const hashPath = url.hash.slice(0, queryIndex);
		const hashParams = new URLSearchParams(url.hash.slice(queryIndex + 1));
		const hashChanged = removeSharedEditorParams(hashParams);

		if (hashChanged) {
			const nextHashParams = hashParams.toString();
			url.hash = nextHashParams ? `${hashPath}?${nextHashParams}` : hashPath;
			changed = true;
		}
	}

	if (changed) {
		window.history.replaceState(
			window.history.state,
			document.title,
			`${url.pathname}${url.search}${url.hash}`,
		);
	}
};