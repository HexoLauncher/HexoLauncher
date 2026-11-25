import type { ModDependencyEdge } from '$lib/api';

export const NODE_WIDTH = 180;
export const NODE_HEIGHT = 36;
const GAP_X = 80;
const GAP_Y = 14;
const SWEEPS = 4;

export type GraphNodeState = 'enabled' | 'disabled' | 'missing';

export interface GraphNode {
	id: string;
	label: string;
	state: GraphNodeState;
	attention: boolean;
	x: number;
	y: number;
}

export interface GraphLink {
	from: string;
	to: string;
	kind: ModDependencyEdge['kind'];
	attention: boolean;
	cyclic: boolean;
}

export interface GraphLayout {
	nodes: GraphNode[];
	links: GraphLink[];
	width: number;
	height: number;
}

function push<T>(map: Map<string, T[]>, key: string, value: T) {
	const list = map.get(key);
	if (list) list.push(value);
	else map.set(key, [value]);
}

export const missingNodeId = (dependencyId: string) => `missing:${dependencyId}`;

export function linkTargets(edge: ModDependencyEdge): string[] {
	if (edge.status === 'runtime') return [];
	if (edge.providers.length) return edge.providers.map((provider) => provider.fileName);
	return edge.kind === 'incompatible' ? [] : [missingNodeId(edge.dependencyId)];
}

export function layoutDependencyGraph(
	edges: ModDependencyEdge[],
	names: Map<string, string>,
	needsAttention: (edge: ModDependencyEdge) => boolean
): GraphLayout {
	const nodes = new Map<string, GraphNode>();
	const links = new Map<string, GraphLink>();

	const addNode = (id: string, label: string, state: GraphNodeState) => {
		let node = nodes.get(id);
		if (!node) {
			node = { id, label, state, attention: false, x: 0, y: 0 };
			nodes.set(id, node);
		} else if (state === 'disabled') {
			node.state = state;
		}
		return node;
	};

	for (const edge of edges) {
		const targets = linkTargets(edge);
		if (!targets.length) continue;
		const source = addNode(edge.fileName, edge.modName, 'enabled');
		const attention = needsAttention(edge);
		for (const target of targets) {
			const provider = edge.providers.find((p) => p.fileName === target);
			const node = provider
				? addNode(target, names.get(target) ?? target, provider.enabled ? 'enabled' : 'disabled')
				: addNode(target, edge.dependencyId, 'missing');
			if (attention) source.attention = node.attention = true;
			const key = `${source.id}\n${target}\n${edge.kind}`;
			const link = links.get(key);
			if (link) link.attention ||= attention;
			else
				links.set(key, { from: source.id, to: target, kind: edge.kind, attention, cyclic: false });
		}
	}

	const outgoing = new Map<string, GraphLink[]>();
	for (const link of links.values()) push(outgoing, link.from, link);

	const ids = [...nodes.keys()].sort((a, b) =>
		nodes.get(a)!.label.localeCompare(nodes.get(b)!.label)
	);
	const visiting = new Set<string>();
	const done = new Set<string>();
	const postorder: string[] = [];
	const visit = (id: string) => {
		visiting.add(id);
		for (const link of outgoing.get(id) ?? []) {
			if (visiting.has(link.to)) link.cyclic = true;
			else if (!done.has(link.to)) visit(link.to);
		}
		visiting.delete(id);
		done.add(id);
		postorder.push(id);
	};
	for (const id of ids) if (!done.has(id)) visit(id);

	const rank = new Map(ids.map((id) => [id, 0]));
	for (const id of postorder.reverse()) {
		for (const link of outgoing.get(id) ?? []) {
			if (!link.cyclic) rank.set(link.to, Math.max(rank.get(link.to)!, rank.get(id)! + 1));
		}
	}

	const columns: string[][] = [];
	for (const id of ids) (columns[rank.get(id)!] ??= []).push(id);

	const forward = new Map<string, string[]>();
	const backward = new Map<string, string[]>();
	for (const link of links.values()) {
		if (link.cyclic) continue;
		push(forward, link.from, link.to);
		push(backward, link.to, link.from);
	}

	const position = new Map<string, number>();
	const record = (column: string[]) => column.forEach((id, index) => position.set(id, index));
	columns.forEach(record);
	const reorder = (column: string[], neighbours: Map<string, string[]>) => {
		const weight = new Map(
			column.map((id) => {
				const linked = neighbours.get(id) ?? [];
				const sum = linked.reduce((total, other) => total + position.get(other)!, 0);
				return [id, linked.length ? sum / linked.length : position.get(id)!];
			})
		);
		column.sort((a, b) => weight.get(a)! - weight.get(b)!);
		record(column);
	};
	for (let sweep = 0; sweep < SWEEPS; sweep++) {
		for (let c = 1; c < columns.length; c++) reorder(columns[c], backward);
		for (let c = columns.length - 2; c >= 0; c--) reorder(columns[c], forward);
	}

	let tallest = 0;
	columns.forEach((column, c) => {
		tallest = Math.max(tallest, column.length);
		column.forEach((id, index) => {
			const node = nodes.get(id)!;
			node.x = c * (NODE_WIDTH + GAP_X);
			node.y = index * (NODE_HEIGHT + GAP_Y);
		});
	});

	return {
		nodes: [...nodes.values()],
		links: [...links.values()],
		width: Math.max(0, columns.length * (NODE_WIDTH + GAP_X) - GAP_X),
		height: Math.max(0, tallest * (NODE_HEIGHT + GAP_Y) - GAP_Y)
	};
}
