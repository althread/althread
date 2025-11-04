/** @jsxImportSource solid-js */
import { createSignal, Show, For, onMount } from 'solid-js';
import { WebPackageManager, type PackageInfo, type AltTomlConfig } from '@utils/packageManager';
import type { FileSystemEntry } from '@components/fileexplorer/FileExplorer';

interface PackageManagerDialogProps {
  isOpen: boolean;
  onClose: () => void;
  fileSystem: FileSystemEntry[];
  setFileSystem: (fs: FileSystemEntry[]) => void;
}

export default function PackageManagerDialog(props: PackageManagerDialogProps) {
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

  // Initialize package manager when dialog opens
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

      console.log(`🔄 Adding dependency: ${packageName()}@${packageVersion()}`);

      const isValid = await pm.validatePackageName(packageName());
      if (!isValid) {
        setError('Invalid package name format');
        return;
      }

      const added = await pm.addDependency(packageName(), packageVersion());
      if (added) {
        setSuccess(`Added dependency: ${packageName()}@${packageVersion()}`);
        console.log(`✅ Dependency added, reloading state...`);
        
        setPackageName('');
        setPackageVersion('latest');
        
        // Reload the current state to refresh the dependencies list
        await loadCurrentState();
        console.log(`🔄 State reloaded, current config:`, altTomlConfig());
      } else {
        setError('Failed to add dependency');
      }
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to add dependency';
      console.error('❌ Add dependency failed:', errorMessage);
      setError(errorMessage);
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
      
      console.log('🔄 Starting dependency installation...');
      await pm.installDependencies();
      setSuccess('Dependencies installed successfully!');
      console.log('✅ Dependencies installed successfully!');
      await loadCurrentState();
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to install dependencies';
      console.error('❌ Installation failed:', errorMessage);
      setError(errorMessage);
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

  const clearCache = async () => {
    const pm = packageManager();
    if (!pm) return;

    try {
      setLoading(true);
      setError(null);
      
      pm.clearCache();
      setSuccess('Package cache cleared');
      await loadCurrentState();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to clear cache');
    } finally {
      setLoading(false);
    }
  };

  const clearMessages = () => {
    setError(null);
    setSuccess(null);
  };

  return (
    <Show when={props.isOpen}>
      <div class="modal-overlay" onClick={props.onClose}>
        <div class="modal-content package-manager-dialog" onClick={(e) => e.stopPropagation()}>
          <div class="modal-header">
            <h2>Package Manager</h2>
            <button class="close-button" onClick={props.onClose}>×</button>
          </div>
          
          <div class="modal-body">
            <Show when={loading()}>
              <div class="loading-spinner">Loading...</div>
            </Show>
            
            <Show when={error()}>
              <div class="error-message">
                {error()}
                <button class="close-msg" onClick={clearMessages}>×</button>
              </div>
            </Show>
            
            <Show when={success()}>
              <div class="success-message">
                {success()}
                <button class="close-msg" onClick={clearMessages}>×</button>
              </div>
            </Show>

            <div class="package-manager-tabs">
              <div class="tab-content">
                <Show when={!altTomlConfig()}>
                  <div class="init-project-section">
                    <h3>Initialize Project</h3>
                    <p>Create an alt.toml file to manage dependencies</p>
                    
                    <div class="form-group">
                      <label>Project Name:</label>
                      <input
                        type="text"
                        value={projectName()}
                        onInput={(e) => setProjectName(e.target.value)}
                        placeholder="e.g., my-project or github.com/user/repo"
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
                      Initialize Project
                    </button>
                  </div>
                </Show>

                <Show when={altTomlConfig()}>
                  <div class="project-info">
                    <h3>Project: {altTomlConfig()?.package.name}</h3>
                    <p>Version: {altTomlConfig()?.package.version}</p>
                  </div>

                  <div class="add-dependency-section">
                    <h3>Add Dependency</h3>
                    
                    <div class="form-group">
                      <label>Package Name:</label>
                      <input
                        type="text"
                        value={packageName()}
                        onInput={(e) => setPackageName(e.target.value)}
                        placeholder="e.g., github.com/user/repo"
                      />
                    </div>
                    
                    <div class="form-group">
                      <label>Version:</label>
                      <input
                        type="text"
                        value={packageVersion()}
                        onInput={(e) => setPackageVersion(e.target.value)}
                        placeholder="latest"
                      />
                    </div>
                    
                    <button 
                      class="btn btn-primary"
                      onClick={addDependency}
                      disabled={loading()}
                    >
                      Add Dependency
                    </button>
                  </div>

                  <div class="dependencies-section">
                    <h3>Dependencies</h3>
                    <Show when={Object.keys(altTomlConfig()?.dependencies || {}).length > 0}>
                      <div class="dependency-list">
                        <For each={Object.entries(altTomlConfig()?.dependencies || {})}>
                          {([name, version]) => (
                            <div class="dependency-item">
                              <span class="dependency-name">{name}</span>
                              <span class="dependency-version">{version}</span>
                              <button 
                                class="btn btn-danger btn-sm"
                                onClick={() => removePackage(name)}
                                disabled={loading()}
                              >
                                Remove
                              </button>
                            </div>
                          )}
                        </For>
                      </div>
                      
                      <button 
                        class="btn btn-success"
                        onClick={installDependencies}
                        disabled={loading()}
                      >
                        Install Dependencies
                      </button>
                    </Show>
                    
                    <Show when={Object.keys(altTomlConfig()?.dependencies || {}).length === 0}>
                      <p>No dependencies found. Add some dependencies above.</p>
                    </Show>
                  </div>

                  <div class="cache-section">
                    <h3>Package Cache</h3>
                    <Show when={cachedPackages().length > 0}>
                      <div class="cached-packages">
                        <For each={cachedPackages()}>
                          {(pkg) => (
                            <div class="cached-package">
                              <span class="package-name">{pkg.name}</span>
                              <span class="package-version">{pkg.version}</span>
                              <span class="package-type" data-type={pkg.type}>{pkg.type}</span>
                            </div>
                          )}
                        </For>
                      </div>
                      
                      <button 
                        class="btn btn-warning"
                        onClick={clearCache}
                        disabled={loading()}
                      >
                        Clear Cache
                      </button>
                    </Show>
                    
                    <Show when={cachedPackages().length === 0}>
                      <p>No cached packages found.</p>
                    </Show>
                  </div>
                </Show>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Show>
  );
}
