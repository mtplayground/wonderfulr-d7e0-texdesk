const starterItems = [
  "Tauri Rust core",
  "React and TypeScript frontend",
  "Vite development server on port 8080",
  "Environment example for workspace and LaTeX toolchain paths",
];

export default function App() {
  return (
    <main className="app-shell">
      <section className="intro">
        <p className="eyebrow">Desktop LaTeX workspace</p>
        <h1>Tauri + React scaffold is ready</h1>
        <p className="summary">
          This baseline provides the native shell and frontend entry points that
          later issues will extend into the file tree, editor, compile flow, and
          PDF preview.
        </p>
      </section>

      <section className="status-panel" aria-label="Scaffold status">
        <h2>Included in issue #1</h2>
        <ul>
          {starterItems.map((item) => (
            <li key={item}>{item}</li>
          ))}
        </ul>
      </section>
    </main>
  );
}
