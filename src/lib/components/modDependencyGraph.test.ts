import { expect, test } from 'vitest';
import type { ModDependencyEdge } from '$lib/api';
import { layoutDependencyGraph, missingNodeId } from './modDependencyGraph';

const edge = (fileName: string, dependencyId: string, provider?: string): ModDependencyEdge => ({
	fileName,
	modId: fileName,
	modName: fileName,
	dependencyId,
	requirement: null,
	kind: 'required',
	status: provider ? 'installed' : 'missing',
	providers: provider ? [{ fileName: provider, version: '1', enabled: true }] : []
});

test('places dependencies in later columns and marks the edge that closes a cycle', () => {
	const layout = layoutDependencyGraph(
		[
			edge('a.jar', 'b', 'b.jar'),
			edge('b.jar', 'c', 'c.jar'),
			edge('c.jar', 'b', 'b.jar'),
			edge('a.jar', 'gone'),
			{ ...edge('a.jar', 'minecraft'), status: 'runtime' }
		],
		new Map(),
		(e) => e.status === 'missing'
	);
	const x = (id: string) => layout.nodes.find((node) => node.id === id)!.x;

	expect(layout.nodes.map((node) => node.id).sort()).toEqual(
		['a.jar', 'b.jar', 'c.jar', missingNodeId('gone')].sort()
	);
	expect(x('a.jar')).toBeLessThan(x('b.jar'));
	expect(x('b.jar')).toBeLessThan(x('c.jar'));
	expect(layout.links.filter((link) => link.cyclic)).toEqual([
		expect.objectContaining({ from: 'c.jar', to: 'b.jar' })
	]);
	expect(layout.nodes.find((node) => node.id === missingNodeId('gone'))?.attention).toBe(true);
});
