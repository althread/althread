/** @jsxImportSource solid-js */
import { createSignal, Show, For, onMount } from 'solid-js';
import { WebPackageManager, type PackageInfo, type AltTomlConfig } from '@utils/packageManager';
import type { FileSystemEntry } from '@components/fileexplorer/FileExplorer';
import './PackageManagerView.css';

interface PackageManagerViewProps {
  fileSystem: FileSystemEntry[];
  setFileSystem: (fs: FileSystemEntry[]) => void;
}

export default function PackageManagerView(props: PackageManagerViewProps) {
  const [packageManager, setPackageManager] = createSignal<WebPackageManager | null>(null);
  const [altTomlConfig, setAltTomlConfig] = createSignal<AltTomlConfig | null>(null);
  const [cachedPackages, setCachedPackages] = createSignal<PackageInfo[]>([]);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [success, setSuccess] = createSignal<string | null>(null);

  // Form state
  const [packageName, setPackageName] = createSignal('');
  const [packageVersion, setPackageVersion] = createSignal('latest');
  const [projectName, setProjectName] = createSignal('');
  const [projectVersion, setProjectVersion] = createSignal('0.1.0');

  // Initialize package manager
  onMount(() => {
    const pm = new WebPackageManager(props.fileSystem, props.setFileSystem);
    setPackageManager(pm);
    loadCurrentState();
  });

  const loadCurrentState = async () => {
    const pm = packageManager();
    if (!pm) return;

    try {
      setLoading(true);
      const config = await pm.parseAltToml();
      setAltTomlConfig(config);
      
      if (config) {
        setProjectName(config.package.name);
        setProjectVersion(config.package.version);
      }
      
      setCachedPackages(pm.getCachedPackages());
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load package state');
    } finally {
      setLoading(false);
    }
  };

  const initializeProject = async () => {
    const pm = packageManager();
    if (!pm) return;

    try {
      setLoading(true);
      setError(null);
      
      if (!projectName().trim()) {
        setError('Project name is required');
        return;
      }

      const isValid = await pm.validatePackageName(projectName());
      if (!isValid) {
        setError('Invalid project name format');
        return;
      }

      await pm.initializeProject(projectName(), projectVersion());
      setSuccess('Project initialized successfully!');
      await loadCurrentState();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to initialize project');
    } finally {
      setLoading(false);
    }
  };

  const addDependency = async () => {
    const pm = packageManager();
    if (!pm) return;

    try {
      setLoading(true);
      setError(null);
      
      if (!packageName().trim()) {
        setError('Package name is required');
        return;
      }

      const isValid = await pm.validatePackageName(packageName());
      if (!isValid) {
        setError('Invalid package name format');
        return;
      }

      const added = await pm.addDependency(packageName(), packageVersion());
      if (added) {
        setSuccess(`Added dependency: ${packageName()}@${packageVersion()}`);
        setPackageName('');
        setPackageVersion('latest');
        await loadCurrentState();
      } else {
        setError('Failed to add dependency');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to add dependency');
    } finally {
      setLoading(false);
    }
  };

  const installDependencies = async () => {
    const pm = packageManager();
    if (!pm) return;

    try {
      setLoading(true);
      setError(null);
      
      await pm.installDependencies();
      setSuccess('Dependencies installed successfully!');
      await loadCurrentState();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to install dependencies');
    } finally {
      setLoading(false);
    }
  };

  const removePackage = async (packageName: string) => {
    const pm = packageManager();
    if (!pm) return;

    try {
      setLoading(true);
      setError(null);
      
      await pm.removePackage(packageName);
      setSuccess(`Removed package: ${packageName}`);
      await loadCurrentState();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to remove package');
    } finally {
      setLoading(false);
    }
  };

  const clearMessages = () => {
    setError(null);
    setSuccess(null);
  };

  return (
    <div class="package-manager-view">
      <div class="package-manager-header">
        <h3>Package Manager</h3>
      </div>

      <div class="package-manager-content">
        <Show when={loading()}>
          <div class="loading-spinner">
            <i class="codicon codicon-loading codicon-modifier-spin"></i>
            Loading...
          </div>
        </Show>

        <Show when={error()}>
          <div class="error-message">
            <i class="codicon codicon-error"></i>
            {error()}
            <button class="close-msg" onClick={clearMessages}>×</button>
          </div>
        </Show>

        <Show when={success()}>
          <div class="success-message">
            <i class="codicon codicon-check"></i>
            {success()}
            <button class="close-msg" onClick={clearMessages}>×</button>
          </div>
        </Show>

        <Show when={!altTomlConfig()}>
          <div class="init-project-section">
            <div class="section-icon">
              <i class="codicon codicon-rocket"></i>
            </div>
            <h4>Initialize Project</h4>
            <p>Create an alt.toml file to manage dependencies</p>
            
            <div class="form-group">
              <label>Project Name:</label>
              <input
                type="text"
                value={projectName()}
                onInput={(e) => setProjectName(e.target.value)}
                placeholder="e.g., my-project"
              />
            </div>
            
            <div class="form-group">
              <label>Version:</label>
              <input
                type="text"
                value={projectVersion()}
                onInput={(e) => setProjectVersion(e.target.value)}
                placeholder="0.1.0"
              />
            </div>
            
            <button 
              class="btn btn-primary"
              onClick={initializeProject}
              disabled={loading()}
            >
              <i class="codicon codicon-rocket"></i>
              Initialize Project
            </button>
          </div>
        </Show>

        <Show when={altTomlConfig()}>
          <div class="project-info">
            <div class="project-header">
              <div class="project-icon">
                <i class="codicon codicon-package"></i>
              </div>
              <div class="project-details">
                <div class="project-name">{altTomlConfig()?.package.name}</div>
                <div class="project-version">v{altTomlConfig()?.package.version}</div>
              </div>
            </div>
          </div>

          <div class="add-dependency-section">
            <div class="section-header">
              <div class="section-icon">
                <i class="codicon codicon-plus"></i>
              </div>
              <h4>Add Dependency</h4>
            </div>
            
            <div class="form-group">
              <input
                type="text"
                value={packageName()}
                onInput={(e) => setPackageName(e.target.value)}
                placeholder="Package name (e.g., github.com/user/repo)"
              />
            </div>
            
            <div class="form-group">
              <input
                type="text"
                value={packageVersion()}
                onInput={(e) => setPackageVersion(e.target.value)}
                placeholder="Version (e.g., latest, 1.0.0)"
              />
            </div>
            
            <button 
              class="btn btn-primary"
              onClick={addDependency}
              disabled={loading()}
            >
              <i class="codicon codicon-plus"></i>
              Add Dependency
            </button>
          </div>

          <div class="dependencies-section">
            <div class="section-header">
              <div class="section-icon">
                <i class="codicon codicon-package"></i>
              </div>
              <h4>Dependencies ({Object.keys(altTomlConfig()?.dependencies || {}).length})</h4>
              <Show when={Object.keys(altTomlConfig()?.dependencies || {}).length > 0}>
                <button 
                  class="btn btn-success btn-sm"
                  onClick={installDependencies}
                  disabled={loading()}
                  title="Install all dependencies"
                >
                  <i class="codicon codicon-cloud-download"></i>
                  Install All
                </button>
              </Show>
            </div>

            <Show when={Object.keys(altTomlConfig()?.dependencies || {}).length > 0}>
              <div class="dependency-list">
                <For each={Object.entries(altTomlConfig()?.dependencies || {})}>
                  {([name, version]) => (
                    <div class="dependency-item">
                      <div class="dependency-icon">
                        <i class="codicon codicon-package"></i>
                      </div>
                      <div class="dependency-info">
                        <div class="dependency-name">{name}</div>
                        <div class="dependency-version">{version}</div>
                      </div>
                      <button 
                        class="btn btn-danger btn-sm"
                        onClick={() => removePackage(name)}
                        disabled={loading()}
                        title="Remove dependency"
                      >
                        <i class="codicon codicon-trash"></i>
                      </button>
                    </div>
                  )}
                </For>
              </div>
            </Show>
            
            <Show when={Object.keys(altTomlConfig()?.dependencies || {}).length === 0}>
              <div class="empty-state">
                <i class="codicon codicon-package"></i>
                <p>No dependencies yet</p>
                <span>Add dependencies above to get started</span>
              </div>
            </Show>
          </div>

          <Show when={cachedPackages().length > 0}>
            <div class="cache-section">
              <div class="section-header">
                <div class="section-icon">
                  <i class="codicon codicon-database"></i>
                </div>
                <h4>Package Cache ({cachedPackages().length})</h4>
              </div>
              <div class="cached-packages">
                <For each={cachedPackages()}>
                  {(pkg) => (
                    <div class="cached-package">
                      <div class="package-icon">
                        <i class="codicon codicon-package"></i>
                      </div>
                      <div class="package-info">
                        <div class="package-name">{pkg.name}</div>
                        <div class="package-version">{pkg.version}</div>
                      </div>
                      <span class="package-type" data-type={pkg.type}>{pkg.type}</span>
                    </div>
                  )}
                </For>
              </div>
            </div>
          </Show>
        </Show>
      </div>
    </div>
  );
}
