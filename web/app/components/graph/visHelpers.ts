import type { Network } from "vis-network";

/**
 * Pans the view (keeping the zoom level) so that the node is visible.
 * Does nothing when the node is already comfortably inside the viewport.
 */
export function revealNode(
	network: Network,
	container: HTMLElement | undefined,
	nodeId: string | number,
) {
	if (!container) return;
	const position = network.getPosition(nodeId as any);
	if (!position) return;
	const dom = network.canvasToDOM(position);
	const margin = 40;
	const inside =
		dom.x >= margin &&
		dom.y >= margin &&
		dom.x <= container.clientWidth - margin &&
		dom.y <= container.clientHeight - margin;
	if (inside) return;
	network.moveTo({
		position,
		animation: { duration: 300, easingFunction: "easeInOutQuad" },
	});
}

export function createGraphToolbarHandlers(
	getNetwork: () => Network | null,
	getContainer: () => HTMLDivElement | undefined,
	toggleMaximized: () => void,
	toggleDetails: () => void = () => {},
) {
	const handleMaximize = () => {
		toggleMaximized();
	};

	const handleDetails = () => {
		toggleDetails();
	};

	const handleRecenter = () => {
		const network = getNetwork();
		if (network) {
			network.fit();
		}
	};

	const handleDownload = () => {
		const network = getNetwork();
		const container = getContainer();
		if (!network || !container) return;

		const canvas = container.querySelector("canvas");
		if (canvas instanceof HTMLCanvasElement) {
			const dataURL = canvas.toDataURL("image/png");
			const link = document.createElement("a");
			link.href = dataURL;
			link.download = "graph.png";
			document.body.appendChild(link);
			link.click();
			document.body.removeChild(link);
		}
	};

	return {
		handleMaximize,
		handleRecenter,
		handleDownload,
		handleDetails,
	};
}

export function formatState(text: string): string {
	// Escape HTML
	let html = text
		.replace(/&/g, "&amp;")
		.replace(/</g, "&lt;")
		.replace(/>/g, "&gt;")
		.replace(/"/g, "&quot;")
		.replace(/'/g, "&#039;");

	// Bold: *text*
	html = html.replace(/\*([^*]+)\*/g, "<b>$1</b>");

	// Italics: _text_
	html = html.replace(/_([^_]+)_/g, "<i>$1</i>");

	return html;
}
