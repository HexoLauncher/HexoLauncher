import { page } from '@vitest/browser/context';
import { expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { checkModDependencies, type ModDependencyEdge } from '$lib/api';
import { m } from '$lib/paraglide/messages.js';
import ModDependencyDialog from './ModDependencyDialog.svelte';

vi.mock('$lib/api', () => ({ checkModDependencies: vi.fn() }));

test('switches to the dependency graph and shows details for a selected mod', async () => {
	const addonDependency: ModDependencyEdge = {
		fileName: 'addon.jar',
		modId: 'addon',
		modName: 'Test Add-on',
		dependencyId: 'library_alias',
		requirement: '>=1',
		kind: 'required',
		status: 'installed',
		providers: [{ fileName: 'library.jar', version: '1', enabled: true }]
	};
	vi.mocked(checkModDependencies).mockResolvedValue({
		relationships: [
			addonDependency,
			{
				...addonDependency,
				dependencyId: 'missing_mod',
				status: 'missing',
				providers: []
			},
			{
				...addonDependency,
				fileName: 'library.jar',
				modId: 'library',
				modName: 'Test Library',
				dependencyId: 'addon',
				providers: [{ fileName: 'addon.jar', version: '1', enabled: true }]
			}
		],
		unrecognizedFiles: []
	});

	render(ModDependencyDialog, { baseDir: '.', instanceId: 'test', onclose: vi.fn() });

	const list = page.getByRole('button', { name: m.blueprint_view_list() });
	const graph = page.getByRole('button', { name: m.editor_dependency_graph() });
	const alias = page.getByText('library_alias', { exact: true });
	await expect.element(list).toHaveAttribute('aria-pressed', 'true');
	await expect.element(alias).toBeVisible();

	await graph.click();
	await expect.element(graph).toHaveAttribute('aria-pressed', 'true');
	await expect.element(alias).not.toBeInTheDocument();
	await expect.element(page.getByText(m.editor_dependency_graph_hint())).toBeVisible();

	const addon = page.getByRole('button', { name: 'Test Add-on' });
	await expect.element(page.getByRole('button', { name: 'Test Library' })).toBeVisible();
	await expect.element(page.getByRole('button', { name: 'missing_mod' })).toBeVisible();
	await addon.click();
	await expect.element(addon).toHaveAttribute('aria-pressed', 'true');
	await expect.element(alias).toBeVisible();
	await expect.element(page.getByText(m.editor_dependency_dependents())).toBeVisible();

	await page.getByRole('checkbox', { name: m.editor_dependency_issues_only() }).click();
	await expect.element(page.getByRole('button', { name: 'Test Library' })).not.toBeInTheDocument();
	await expect.element(page.getByRole('button', { name: 'missing_mod' })).toBeVisible();
	await expect.element(alias).not.toBeInTheDocument();
	await list.click();
	await expect.element(page.getByText('missing_mod', { exact: true })).toBeVisible();
	await expect.element(alias).not.toBeInTheDocument();
});
