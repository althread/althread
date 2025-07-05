import type { FileSystemEntry } from '@components/fileexplorer/FileExplorer';
import { saveFileContent, loadFileContent } from './storage';
import { findFileByPath } from './fileSystemUtils';

// Package management for web editor
export interface PackageInfo {
  name: string;
  version: string;
  url?: string;
  type: 'local' | 'remote';
  cached?: boolean;
  files?: Record<string, string>; // For cached remote packages
}

export interface AltTomlConfig {
  package: {
    name: string;
    version: string;
  };
  dependencies: Record<string, string>;
  'dev-dependencies'?: Record<string, string>;
}

// Package cache storage keys
const PACKAGE_CACHE_KEY = 'althread-package-cache';
const PACKAGE_METADATA_KEY = 'althread-package-metadata';

export class WebPackageManager {
  private packageCache: Map<string, PackageInfo> = new Map();
  private fileSystem: FileSystemEntry[];
  private setFileSystem: (fs: FileSystemEntry[]) => void;

  constructor(fileSystem: FileSystemEntry[], setFileSystem: (fs: FileSystemEntry[]) => void) {
    this.fileSystem = fileSystem;
    this.setFileSystem = setFileSystem;
    this.loadPackageCache();
  }

  // Load package cache from localStorage
  private loadPackageCache(): void {
    try {
      const cached = localStorage.getItem(PACKAGE_CACHE_KEY);
      if (cached) {
        const cacheData = JSON.parse(cached);
        this.packageCache = new Map(Object.entries(cacheData));
      }
    } catch (error) {
      console.error('Failed to load package cache:', error);
    }
  }

  // Save package cache to localStorage
  private savePackageCache(): void {
    try {
      const cacheData = Object.fromEntries(this.packageCache.entries());
      localStorage.setItem(PACKAGE_CACHE_KEY, JSON.stringify(cacheData));
    } catch (error) {
      console.error('Failed to save package cache:', error);
    }
  }

  // Initialize a new alt.toml file
  public async initializeProject(packageName: string, version: string = '0.1.0'): Promise<void> {
    // Import and initialize the WASM module
    const wasmModule = await import('../../pkg/althread_web');
    await wasmModule.default(); // Initialize the WASM module
    
    const tomlContent = wasmModule.create_alt_toml(packageName, version);
    
    // Check if alt.toml already exists
    const existingFile = findFileByPath(this.fileSystem, 'alt.toml');
    if (existingFile) {
      // Just update the content
      saveFileContent('alt.toml', tomlContent);
      return;
    }
    
    // Add alt.toml to the file system
    const newFile: FileSystemEntry = {
      id: crypto.randomUUID(),
      name: 'alt.toml',
      type: 'file'
    };

    // Add to root of file system
    const updatedFileSystem = [...this.fileSystem, newFile];
    
    // Update both the internal reference and trigger UI update
    this.fileSystem = updatedFileSystem;
    this.setFileSystem(updatedFileSystem);
    
    // Save the content
    saveFileContent('alt.toml', tomlContent);
    
    console.log('✓ Project initialized with alt.toml');
  }

  // Parse alt.toml file and return dependencies
  public async parseAltToml(): Promise<AltTomlConfig | null> {
    console.log('🔍 Parsing alt.toml...');
    console.log('Current file system:', this.fileSystem.map(f => f.name));
    
    const altTomlFile = findFileByPath(this.fileSystem, 'alt.toml');
    if (!altTomlFile) {
      console.log('❌ No alt.toml file found in file system');
      return null;
    }

    console.log('✓ Found alt.toml file');
    const content = loadFileContent('alt.toml');
    if (!content) {
      console.log('❌ No content loaded from alt.toml');
      return null;
    }

    console.log('✓ Loaded alt.toml content:', content.substring(0, 100) + '...');

    try {
      // Import and initialize the WASM module
      const wasmModule = await import('../../pkg/althread_web');
      await wasmModule.default(); // Initialize the WASM module
      
      const dependenciesResult = wasmModule.parse_dependencies_from_toml(content);
      
      // Convert Map to plain object if needed
      let dependencies: Record<string, string> = {};
      if (dependenciesResult instanceof Map) {
        dependencies = Object.fromEntries(dependenciesResult.entries());
      } else if (typeof dependenciesResult === 'object' && dependenciesResult !== null) {
        dependencies = dependenciesResult as Record<string, string>;
      }
      
      console.log('✓ Parsed dependencies:', dependencies);
      
      // Parse the basic package info manually (simple TOML parsing)
      const packageMatch = content.match(/\[package\]\s*name\s*=\s*"([^"]+)"\s*version\s*=\s*"([^"]+)"/s);
      if (!packageMatch) {
        console.log('❌ Invalid alt.toml format - no package section found');
        throw new Error('Invalid alt.toml format');
      }

      console.log('✓ Successfully parsed alt.toml');
      return {
        package: {
          name: packageMatch[1],
          version: packageMatch[2]
        },
        dependencies: dependencies || {}
      };
    } catch (error) {
      console.error('❌ Failed to parse alt.toml:', error);
      return null;
    }
  }

  // Add a dependency to alt.toml
  public async addDependency(packageName: string, version: string): Promise<boolean> {
    console.log(`📦 Adding dependency: ${packageName}@${version}`);
    
    const altTomlFile = findFileByPath(this.fileSystem, 'alt.toml');
    if (!altTomlFile) {
      console.error('❌ No alt.toml file found');
      return false;
    }

    try {
      const content = loadFileContent('alt.toml');
      console.log('📄 Current alt.toml content:', content);
      
      // Import and initialize the WASM module
      const wasmModule = await import('../../pkg/althread_web');
      await wasmModule.default(); // Initialize the WASM module
      
      // The WASM function returns a Result<String, JsValue>, so we need to handle potential errors
      let updatedContent: string;
      try {
        updatedContent = wasmModule.add_dependency_to_toml(content, packageName, version);
      } catch (wasmError) {
        console.error('❌ WASM function failed:', wasmError);
        throw new Error(`Failed to update alt.toml: ${wasmError}`);
      }
      
      console.log('📄 Updated alt.toml content:', updatedContent);
      
      // Save the updated content
      saveFileContent('alt.toml', updatedContent);
      console.log('✅ Dependency added successfully');
      return true;
    } catch (error) {
      console.error('❌ Failed to add dependency:', error);
      throw error; // Re-throw to show in UI
    }
  }

  // Install dependencies (fetch remote packages)
  public async installDependencies(): Promise<void> {
    const config = await this.parseAltToml();
    if (!config) {
      throw new Error('No alt.toml file found or invalid format');
    }

    const installPromises = Object.entries(config.dependencies).map(([name, version]) => 
      this.installPackage(name, version)
    );

    await Promise.all(installPromises);
  }

  // Install a single package
  public async installPackage(packageName: string, version: string): Promise<void> {
    // Check if already cached
    // @{version}
    const cacheKey = `${packageName}`;
    if (this.packageCache.has(cacheKey)) {
      console.log(`Using cached package: ${packageName}`);
      await this.loadPackageIntoFileSystem(packageName, this.packageCache.get(cacheKey)!);
      return;
    }

    console.log(`Installing package: ${packageName}@${version}`);

    // Fetch from remote
    const packageInfo = await this.fetchRemotePackage(packageName, version);
    if (packageInfo) {
      // Cache the package
      this.packageCache.set(cacheKey, packageInfo);
      this.savePackageCache();
      
      // Load into file system
      await this.loadPackageIntoFileSystem(packageName, packageInfo);
      console.log(`✓ Successfully installed: ${packageName}`);
    } else {
      throw new Error(`Failed to install package: ${packageName}`);
    }
  }

  // Fetch a remote package from GitHub
  private async fetchRemotePackage(packageName: string, version: string): Promise<PackageInfo | null> {
    console.log('🚀 fetchRemotePackage called with:', packageName, version);
    try {
        const parts = packageName.replace('github.com/', '').split('/');
      console.log('🔍 Debug info:');
      console.log('  packageName:', packageName);
      console.log('  parts:', parts);
      console.log('  parts.length:', parts.length);
      

      // Parse GitHub URL format: github.com/user/repo
      if (!packageName.startsWith('github.com/')) {
        throw new Error('Only GitHub packages are supported currently');
      }

    //   if (parts.length < 2) {
    //     throw new Error('Invalid GitHub package format: expected github.com/user/repo');
    //   }
      
      const [user, repo] = parts;
      console.log('  user:', user);
      console.log('  repo:', repo);
      console.log('  user empty?', !user);
      console.log('  repo empty?', !repo);
      
      if (!user || !repo) {
        throw new Error(`Invalid GitHub package format - user: "${user}", repo: "${repo}"`);
      }

      console.log(`Fetching package: ${packageName} (${user}/${repo})`);

      // Special case: if this is a test package, use mock data
      if (packageName === 'github.com/althread/test-package') {
        return this.createMockPackage(packageName, version);
      }

      const files: Record<string, string> = {};

      // Recursively fetch all .alt files
      await this.fetchDirectoryContents(user, repo, '', files);

      if (Object.keys(files).length === 0) {
        console.warn(`No .alt files found in ${packageName}, creating minimal package`);
        // Create a minimal package with just a README
        files['README.md'] = `# ${packageName}\n\nThis package was imported from GitHub but contains no .alt files.`;
      }

      console.log(`Fetched ${Object.keys(files).length} files from ${packageName}`);

      return {
        name: packageName,
        version,
        url: `https://github.com/${user}/${repo}`,
        type: 'remote',
        cached: true,
        files
      };
    } catch (error) {
      console.error(`Failed to fetch remote package ${packageName}:`, error);
      throw error; // Re-throw to show error in UI
    }
  }

  // Create a mock package for testing
  private createMockPackage(packageName: string, version: string): PackageInfo {
    return {
      name: packageName,
      version,
      url: `https://github.com/${packageName.replace('github.com/', '')}`,
      type: 'remote',
      cached: true,
      files: {
        'main.alt': `// Mock package: ${packageName}
process main() {
  print("Hello from ${packageName}!");
}

export function greet(name: string) {
  print("Hello, " + name + "!");
}
`,
        'utils/math.alt': `// Math utilities
export function add(a: int, b: int) -> int {
  return a + b;
}

export function multiply(a: int, b: int) -> int {
  return a * b;
}
`,
        'alt.toml': `[package]
name = "${packageName}"
version = "${version}"

[dependencies]

[dev-dependencies]
`
      }
    };
  }

  // Recursively fetch directory contents from GitHub
  private async fetchDirectoryContents(
    user: string, 
    repo: string, 
    path: string, 
    files: Record<string, string>
  ): Promise<void> {
    const apiUrl = `https://api.github.com/repos/${user}/${repo}/contents/${path}`;
    console.log(`Fetching directory: ${apiUrl}`);
    
    try {
      const response = await fetch(apiUrl);
      
      if (!response.ok) {
        console.warn(`Failed to fetch directory ${path}: ${response.statusText}`);
        return;
      }

      const contents = await response.json();
      
      if (!Array.isArray(contents)) {
        console.warn(`Expected array but got ${typeof contents} for ${path}`);
        return;
      }
      
      for (const item of contents) {
        if (item.type === 'file' && (item.name.endsWith('.alt') || item.name === 'alt.toml')) {
          // Fetch file content
          console.log(`Fetching file: ${item.name}`);
          try {
            const fileResponse = await fetch(item.download_url);
            if (fileResponse.ok) {
              const content = await fileResponse.text();
              const filePath = path ? `${path}/${item.name}` : item.name;
              files[filePath] = content;
              console.log(`✓ Fetched file: ${filePath}`);
            }
          } catch (error) {
            console.warn(`Failed to fetch file ${item.name}:`, error);
          }
        } else if (item.type === 'dir') {
          // Recursively fetch directory contents
          const dirPath = path ? `${path}/${item.name}` : item.name;
          await this.fetchDirectoryContents(user, repo, dirPath, files);
        }
      }
    } catch (error) {
      console.error(`Error fetching directory ${path}:`, error);
    }
  }

  // Load a package into the file system
  private async loadPackageIntoFileSystem(packageName: string, packageInfo: PackageInfo): Promise<void> {
    if (!packageInfo.files) {
      console.warn(`No files found in package: ${packageName}`);
      return;
    }

    console.log(`Loading package into file system: ${packageName}`);

    // Create a dependencies directory if it doesn't exist
    let depsDir = this.fileSystem.find(entry => entry.name === 'deps' && entry.type === 'directory');
    if (!depsDir) {
      depsDir = {
        id: crypto.randomUUID(),
        name: 'deps',
        type: 'directory',
        children: []
      };
      this.fileSystem.push(depsDir);
    }

    // Create package directory
    const packageDirName = packageName.replace(/[^a-zA-Z0-9]/g, '_');
    let packageDir = depsDir.children?.find(entry => entry.name === packageDirName);
    if (!packageDir) {
      packageDir = {
        id: crypto.randomUUID(),
        name: packageDirName,
        type: 'directory',
        children: []
      };
      depsDir.children = depsDir.children || [];
      depsDir.children.push(packageDir);
    }

    // Clear existing files in package directory
    packageDir.children = [];

    // Add all package files
    for (const [filePath, content] of Object.entries(packageInfo.files)) {
      const pathParts = filePath.split('/');
      let currentDir = packageDir;

      // Create nested directories
      for (let i = 0; i < pathParts.length - 1; i++) {
        const dirName = pathParts[i];
        let subDir = currentDir.children?.find(entry => entry.name === dirName && entry.type === 'directory');
        if (!subDir) {
          subDir = {
            id: crypto.randomUUID(),
            name: dirName,
            type: 'directory',
            children: []
          };
          currentDir.children = currentDir.children || [];
          currentDir.children.push(subDir);
        }
        currentDir = subDir;
      }

      // Add the file
      const fileName = pathParts[pathParts.length - 1];
      const fileEntry: FileSystemEntry = {
        id: crypto.randomUUID(),
        name: fileName,
        type: 'file'
      };

      currentDir.children = currentDir.children || [];
      currentDir.children.push(fileEntry);

      // Save file content
      const fullPath = `deps/${packageDirName}/${filePath}`;
      saveFileContent(fullPath, content);
      console.log(`✓ Added file: ${fullPath}`);
    }

    // Trigger UI update by calling setFileSystem
    this.fileSystem = [...this.fileSystem]; // Update internal reference
    this.setFileSystem(this.fileSystem);
    console.log(`✓ Package loaded: ${packageName} (${Object.keys(packageInfo.files).length} files)`);
  }

  // Get cached packages
  public getCachedPackages(): PackageInfo[] {
    return Array.from(this.packageCache.values());
  }

  // Clear package cache
  public clearCache(): void {
    this.packageCache.clear();
    localStorage.removeItem(PACKAGE_CACHE_KEY);
    localStorage.removeItem(PACKAGE_METADATA_KEY);
  }

  // Remove a package from cache and file system
  public async removePackage(packageName: string): Promise<void> {
    // Remove from cache
    const keysToRemove = Array.from(this.packageCache.keys()).filter(key => key.startsWith(packageName));
    keysToRemove.forEach(key => this.packageCache.delete(key));
    this.savePackageCache();

    // Remove from file system
    const depsDir = this.fileSystem.find(entry => entry.name === 'deps' && entry.type === 'directory');
    if (depsDir && depsDir.children) {
      const packageDirName = packageName.replace(/[^a-zA-Z0-9]/g, '_');
      depsDir.children = depsDir.children.filter(entry => entry.name !== packageDirName);
      this.setFileSystem([...this.fileSystem]);
    }

    // Remove from alt.toml
    const config = await this.parseAltToml();
    if (config && config.dependencies[packageName]) {
      delete config.dependencies[packageName];
      // Update alt.toml content
      const altTomlContent = this.generateAltTomlContent(config);
      saveFileContent('alt.toml', altTomlContent);
    }
  }

  // Generate alt.toml content from config
  private generateAltTomlContent(config: AltTomlConfig): string {
    let content = `[package]\n`;
    content += `name = "${config.package.name}"\n`;
    content += `version = "${config.package.version}"\n\n`;
    
    content += `[dependencies]\n`;
    for (const [name, version] of Object.entries(config.dependencies)) {
      content += `"${name}" = "${version}"\n`;
    }
    
    content += `\n[dev-dependencies]\n`;
    if (config['dev-dependencies']) {
      for (const [name, version] of Object.entries(config['dev-dependencies'])) {
        content += `"${name}" = "${version}"\n`;
      }
    }

    return content;
  }

  // Validate package name using WASM
  public async validatePackageName(packageName: string): Promise<boolean> {
    try {
      const wasmModule = await import('../../pkg/althread_web');
      await wasmModule.default(); // Initialize the WASM module
      return wasmModule.validate_package_name(packageName);
    } catch (error) {
      console.error('Failed to validate package name:', error);
      return false;
    }
  }
}
