use anyhow::{Result, bail};
use cranelift_isle::{
    lexer::Pos,
    sema::{RuleId, TermId, TypeId},
    trie_again::BindingId,
};

use crate::{
    debug::{binding_string, constrain_string},
    expand::{Chaining, Expansion},
    program::Program,
    trie::{BindingType, binding_type},
};
use std::{
    fs::File,
    io::{self, Write},
    path::{Component, Path, PathBuf},
};

pub struct ExplorerWriter<'a> {
    prog: &'a Program,
    chaining: &'a Chaining<'a>,
    expansions: &'a Vec<Expansion>,

    root: std::path::PathBuf,
    base: std::path::PathBuf,
    graphs: bool,
    dev: bool,
}

impl<'a> ExplorerWriter<'a> {
    pub fn new(
        root: std::path::PathBuf,
        prog: &'a Program,
        chaining: &'a Chaining<'a>,
        expansions: &'a Vec<Expansion>,
    ) -> Self {
        Self {
            prog,
            chaining,
            expansions,
            root,
            base: PathBuf::new(),
            graphs: false,
            dev: true, // TODO(mbm): configurable dev mode
        }
    }

    pub fn enable_graphs(&mut self) {
        self.graphs = true;
    }

    pub fn write(&mut self) -> Result<()> {
        self.init()?;
        self.write_assets()?;
        self.write_index()?;
        self.write_files()?;
        self.write_types()?;
        self.write_terms()?;
        self.write_rules()?;
        self.write_expansions()?;
        Ok(())
    }

    fn init(&self) -> Result<()> {
        std::fs::create_dir_all(&self.root)?;
        Ok(())
    }

    fn write_assets(&mut self) -> Result<()> {
        // In development mode, setup a symlink.
        if self.dev {
            let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            // TODO(mbm): platform-independent symlink
            let original = crate_root.join("src/assets");
            let link_path = self.abs(&self.assets_dir());
            std::os::unix::fs::symlink(original, link_path)?;
            return Ok(());
        }

        // CSS.
        let style_css = include_bytes!("./assets/style.css");
        let mut output = self.create(&self.style_path())?;
        output.write_all(style_css)?;

        Ok(())
    }

    fn write_index(&mut self) -> Result<()> {
        let mut output = self.create(&PathBuf::from("index.html"))?;
        self.header(&mut output, "ISLE Explorer")?;
        writeln!(
            output,
            r#"
        <menu>
            <li><a href="{files_href}">Files</a></li>
            <li><a href="{types_href}">Types</a></li>
            <li><a href="{terms_href}">Terms</a></li>
            <li><a href="{rules_href}">Rules</a></li>
            <li><a href="{expansions_href}">Expansions</a></li>
        </menu>
        "#,
            files_href = self.link(&self.file_dir()),
            types_href = self.link(&self.types_dir()),
            terms_href = self.link(&self.terms_dir()),
            rules_href = self.link(&self.rules_dir()),
            expansions_href = self.link(&self.expansions_dir()),
        )?;
        self.footer(&mut output)?;
        Ok(())
    }

    fn write_files(&mut self) -> Result<()> {
        self.write_files_index()?;
        for id in 0..self.prog.files.file_names.len() {
            self.write_file(id)?;
        }
        Ok(())
    }

    fn write_files_index(&mut self) -> Result<()> {
        let mut output = self.create(&self.file_dir().join("index.html"))?;
        self.header(&mut output, "Files")?;

        // Files.
        writeln!(output, "<ul>")?;
        for (id, filename) in self.prog.files.file_names.iter().enumerate() {
            writeln!(
                output,
                r#"<li><a href="{link}">{filename}</a></li>"#,
                link = self.link(&self.file_path(id)),
            )?;
        }
        writeln!(output, "</ul>")?;

        self.footer(&mut output)?;
        Ok(())
    }

    fn write_file(&mut self, id: usize) -> Result<()> {
        let mut output = self.create(&self.file_path(id))?;

        // Header.
        let filename = &self.prog.files.file_names[id];
        let title = format!("File: {filename}");
        self.header(&mut output, &title)?;

        // Source code.
        let file_text = &self.prog.files.file_texts[id];

        writeln!(&mut output, "<pre>")?;
        for (i, line) in file_text.lines().enumerate() {
            let n = i + 1;
            writeln!(
                &mut output,
                r#"<code id="{fragment}">{line}</code>"#,
                fragment = self.line_url_fragment(n)
            )?;
        }
        writeln!(&mut output, "</pre>")?;

        // Footer.
        self.footer(&mut output)?;

        Ok(())
    }

    fn write_types(&mut self) -> Result<()> {
        let mut output = self.create(&self.types_dir().join("index.html"))?;
        self.header(&mut output, "Types")?;

        // Types.
        writeln!(
            output,
            r#"
        <table>
            <thead>
                <tr>
                    <th class="id">&num;</th>
                    <th>Name</th>
                    <th>Location</th>
                    <th>Model</th>
                </tr>
            </thead>
            <tbody>
        "#
        )?;
        for ty in &self.prog.tyenv.types {
            writeln!(output, "<tr>")?;
            writeln!(output, r#"<td class="id">{id}</td>"#, id = ty.id().index())?;

            // Name.
            writeln!(output, r"<td>{name}</td>", name = ty.name(&self.prog.tyenv))?;

            // Location.
            if let Some(pos) = ty.pos() {
                writeln!(output, "<td>{pos}</td>", pos = self.pos(pos))?;
            } else {
                writeln!(output, "<td>builtin</td>")?;
            }

            // Model.
            if let Some(model) = self.prog.specenv.type_model.get(&ty.id()) {
                writeln!(output, "<td>{model}</td>")?;
            } else {
                writeln!(output, "<td></td>")?;
            }

            writeln!(output, "</tr>")?;
        }
        writeln!(
            output,
            r#"
            </tbody>
        </table>
        "#
        )?;

        self.footer(&mut output)?;
        Ok(())
    }

    fn write_terms(&mut self) -> Result<()> {
        let mut output = self.create(&self.terms_dir().join("index.html"))?;
        self.header(&mut output, "Terms")?;

        // Terms.
        let term_ids = (0..self.prog.termenv.terms.len()).map(TermId);
        self.write_terms_list(&mut output, term_ids)?;

        self.footer(&mut output)?;
        Ok(())
    }

    fn write_terms_list(
        &self,
        output: &mut dyn Write,
        term_ids: impl Iterator<Item = TermId>,
    ) -> Result<()> {
        writeln!(
            output,
            r#"
        <table>
            <thead>
                <tr>
                    <th class="id">&num;</th>
                    <th>Name</th>
                    <th>Location</th>
                    <th>Spec</th>
                </tr>
            </thead>
            <tbody>
        "#
        )?;
        for term_id in term_ids {
            let term = self.prog.term(term_id);

            writeln!(output, "<tr>")?;
            writeln!(output, r#"<td class="id">{id}</td>"#, id = term.id.index())?;

            // Name.
            writeln!(
                output,
                r"<td>{name}</td>",
                name = self.prog.term_name(term.id)
            )?;

            // Location.
            writeln!(output, "<td>{pos}</td>", pos = self.pos(term.decl_pos))?;

            // Spec.
            if let Some(spec) = self.prog.specenv.term_spec.get(&term.id) {
                writeln!(output, "<td>{pos}</td>", pos = self.pos(spec.pos))?;
            } else if self.chaining.should_chain(term_id) {
                writeln!(output, "<td>chained</td>")?;
            } else {
                writeln!(output, "<td></td>")?;
            }

            writeln!(output, "</tr>")?;
        }
        writeln!(
            output,
            r#"
            </tbody>
        </table>
        "#
        )?;
        Ok(())
    }

    fn write_rules(&mut self) -> Result<()> {
        let mut output = self.create(&self.rules_dir().join("index.html"))?;
        self.header(&mut output, "Rules")?;

        // Rules.
        let rule_ids = (0..self.prog.termenv.rules.len()).map(RuleId);
        self.write_rules_list(&mut output, rule_ids)?;

        self.footer(&mut output)?;
        Ok(())
    }

    fn write_rules_list(
        &self,
        output: &mut dyn Write,
        rule_ids: impl Iterator<Item = RuleId>,
    ) -> Result<()> {
        writeln!(
            output,
            r#"
        <table>
            <thead>
                <tr>
                    <th class="id">&num;</th>
                    <th>Identifier</th>
                </tr>
            </thead>
            <tbody>
        "#
        )?;

        for rule_id in rule_ids {
            writeln!(output, "<tr>")?;
            writeln!(output, r#"<td class="id">{id}</td>"#, id = rule_id.index())?;
            writeln!(
                output,
                "<td>{rule_ref}</td>",
                rule_ref = self.rule_ref(rule_id)
            )?;
            writeln!(output, "</tr>")?;
        }

        writeln!(
            output,
            r#"
            </tbody>
        </table>
        "#
        )?;
        Ok(())
    }

    fn write_expansions(&mut self) -> Result<()> {
        self.write_expansions_index()?;
        for (id, expansion) in self.expansions.iter().enumerate() {
            self.write_expansion(id, expansion)?;
        }
        Ok(())
    }

    fn write_expansions_index(&mut self) -> Result<()> {
        let mut output = self.create(&self.expansions_dir().join("index.html"))?;
        self.header(&mut output, "Expansions")?;

        // Expansions.
        writeln!(
            output,
            r#"
        <table>
            <thead>
                <tr>
                    <th>&num;</th>
                    <th>Root</th>
                    <th>First Rule</th>
                    <th>Tags</th>
                </tr>
            </thead>
            <tbody>
        "#
        )?;
        for (id, expansion) in self.expansions.iter().enumerate() {
            writeln!(output, "<tr>")?;

            // ID
            writeln!(
                output,
                r#"<td><a href="{link}">&num;{id}</a></td>"#,
                link = self.link(&self.expansion_path(id))
            )?;

            // Root
            writeln!(
                output,
                "<td>{term_ref}</td>",
                term_ref = self.term_ref(expansion.term)
            )?;

            // First Rule
            let rule_id = expansion
                .rules
                .first()
                .expect("expansion must have at least one rule");
            writeln!(
                output,
                "<td>{rule_ref}</td>",
                rule_ref = self.rule_ref(*rule_id)
            )?;

            // Tags
            let mut tags: Vec<String> = expansion.tags(self.prog).iter().cloned().collect();
            tags.sort();
            writeln!(output, "<td>{tags}</td>", tags = tags.join(", "))?;

            writeln!(output, "</tr>")?;
        }
        writeln!(
            output,
            r#"
            </tbody>
        </table>
        "#
        )?;

        self.footer(&mut output)?;
        Ok(())
    }

    fn write_expansion(&mut self, id: usize, expansion: &Expansion) -> Result<()> {
        self.write_expansion_index(id, expansion)?;
        if self.graphs {
            self.write_expansion_graph(id, expansion)?;
        }
        Ok(())
    }

    fn write_expansion_index(&mut self, id: usize, expansion: &Expansion) -> Result<()> {
        let mut output = self.create(&self.expansion_path(id))?;

        // Header.
        let title = format!("Expansion: &num;{id}");
        self.header(&mut output, &title)?;

        // Term.
        writeln!(
            output,
            "<p>Term: {term_ref}</p>",
            term_ref = self.term_ref(expansion.term)
        )?;

        // Rules
        writeln!(output, "<h2>Rules</h2>")?;
        self.write_rules_list(&mut output, expansion.rules.iter().copied())?;

        // Negated Rules
        if !expansion.negated.is_empty() {
            writeln!(output, "<h2>Negated</h2>")?;
            self.write_rules_list(&mut output, expansion.negated.iter().copied())?;
        }

        // Terms
        writeln!(output, "<h2>Terms</h2>")?;
        let terms = expansion.terms(self.prog);
        self.write_terms_list(&mut output, terms.into_iter())?;

        // Bindings
        writeln!(output, "<h2>Bindings</h2>")?;
        if self.graphs {
            writeln!(
                output,
                r#"<p>Graph: <a href="{svg_href}">SVG</a>, <a href="{dot_href}">DOT</a>.</p>"#,
                svg_href = self.link(&self.expansion_graph_svg_path(id)),
                dot_href = self.link(&self.expansion_graph_dot_path(id)),
            )?;
        }

        writeln!(
            output,
            r#"
        <table>
            <thead>
                <tr>
                    <th>&num;</th>
        "#
        )?;
        if !expansion.equals.is_empty() {
            writeln!(output, "<th>&equals;</th>")?;
        }
        writeln!(
            output,
            r#"
                    <th>Type</th>
                    <th>Binding</th>
                </tr>
            </thead>
            <tbody>
        "#
        )?;
        let lookup_binding =
            |binding_id: BindingId| expansion.bindings[binding_id.index()].clone().unwrap();
        for (i, binding) in expansion.bindings.iter().enumerate() {
            let id: BindingId = i.try_into().unwrap();
            if let Some(binding) = binding {
                writeln!(output, "<tr>")?;
                let ty = binding_type(binding, expansion.term, self.prog, lookup_binding);

                // ID
                writeln!(output, "<td>{id}</td>", id = id.index())?;

                // Equals
                if let Some(eq) = expansion.equals.find(id)
                    && id != eq
                {
                    write!(output, "<td>&equals; {}</td>", eq.index())?;
                }

                // Type
                writeln!(output, "<td>{ty}</td>", ty = self.binding_type(&ty))?;

                // Binding
                writeln!(
                    output,
                    "<td>{binding}</td>",
                    binding = binding_string(binding, expansion.term, self.prog, lookup_binding)
                )?;

                writeln!(output, "</tr>")?;
            }
        }

        // TODO(mbm): Parameters
        // TODO(mbm): Result

        writeln!(
            output,
            r#"
            </tbody>
        </table>
        "#
        )?;

        // Constraints
        writeln!(output, "<h2>Constraints</h2>")?;
        writeln!(output, "<ul>")?;
        for constrain in &expansion.constraints {
            writeln!(
                output,
                "<li>{constrain}</li>",
                constrain = constrain_string(constrain, &self.prog.tyenv)
            )?;
        }
        writeln!(output, "</ul>")?;

        // Footer.
        self.footer(&mut output)?;

        Ok(())
    }

    fn write_expansion_graph(&mut self, id: usize, expansion: &Expansion) -> Result<()> {
        self.write_expansion_graph_dot(id, expansion)?;
        self.generate_expansion_graph_svg(id)?;
        Ok(())
    }

    fn write_expansion_graph_dot(&mut self, id: usize, expansion: &Expansion) -> Result<()> {
        let mut output = self.create(&self.expansion_graph_dot_path(id))?;

        // Header.
        writeln!(&mut output, "digraph {{")?;
        writeln!(&mut output, "\tnode [shape=box, fontname=monospace];")?;

        // Binding nodes.
        let lookup_binding =
            |binding_id: BindingId| expansion.bindings[binding_id.index()].clone().unwrap();
        for (i, binding) in expansion.bindings.iter().enumerate() {
            if let Some(binding) = binding {
                writeln!(
                    &mut output,
                    "\tb{i} [label=\"{i}: {}\"];",
                    binding_string(binding, expansion.term, self.prog, lookup_binding)
                )?;
            }
        }

        // Edges.
        for (i, binding) in expansion.bindings.iter().enumerate() {
            if let Some(binding) = binding {
                for source in binding.sources() {
                    writeln!(&mut output, "\tb{i} -> b{j};", j = source.index())?;
                }
            }
        }

        writeln!(&mut output, "}}")?;

        Ok(())
    }

    fn generate_expansion_graph_svg(&self, id: usize) -> Result<()> {
        let dot_path = self.expansion_graph_dot_path(id);
        let svg_path = self.expansion_graph_svg_path(id);

        // Invoke graphviz.
        let status = std::process::Command::new("dot")
            .current_dir(&self.root)
            .arg("-Tsvg")
            .arg("-o")
            .arg(svg_path)
            .arg(dot_path)
            .status()?;

        if !status.success() {
            bail!("dot exit status: {status}");
        }

        Ok(())
    }

    fn header(&self, output: &mut dyn Write, title: &str) -> io::Result<()> {
        write!(
            output,
            r#"
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>{title}</title>
    <link rel="stylesheet" href="{style_path}" />
  </head>
  <body>
    <main>
      <h1>{title}</h1>
        "#,
            style_path = self.link(&self.style_path())
        )
    }

    fn footer(&self, output: &mut dyn Write) -> io::Result<()> {
        write!(
            output,
            r#"
    </main>
  </body>
</html>
        "#
        )
    }

    fn binding_type(&self, ty: &BindingType) -> String {
        match ty {
            BindingType::Base(type_id) => self.type_ref(*type_id),
            BindingType::Option(inner) => format!("Option({})", self.binding_type(inner)),
            BindingType::Tuple(inners) => format!(
                "({inners})",
                inners = inners
                    .iter()
                    .map(|inner| self.binding_type(inner))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    fn type_ref(&self, type_id: TypeId) -> String {
        let ty = self.prog.ty(type_id);
        format!(
            r#"<a href="{href}">{name}</a>"#,
            href = self.pos_href(ty.pos().expect("expected position")),
            name = self.prog.type_name(ty.id())
        )
    }

    fn term_ref(&self, term_id: TermId) -> String {
        let term = self.prog.term(term_id);
        format!(
            r#"<a href="{href}">{name}</a>"#,
            href = self.pos_href(term.decl_pos),
            name = self.prog.term_name(term_id)
        )
    }

    fn rule_ref(&self, rule_id: RuleId) -> String {
        let rule = self.prog.rule(rule_id);
        format!(
            r#"<a href="{href}">{identifier}</a>"#,
            href = self.pos_href(rule.pos),
            identifier = rule.identifier(&self.prog.tyenv, &self.prog.files)
        )
    }

    fn pos(&self, pos: Pos) -> String {
        if pos.is_unknown() {
            "&lt;unknown&gt;".to_string()
        } else {
            format!(
                r#"<a href="{href}">{loc}</a>"#,
                href = self.pos_href(pos),
                loc = self.loc(pos)
            )
        }
    }

    fn loc(&self, pos: Pos) -> String {
        let path = PathBuf::from(&self.prog.files.file_names[pos.file]);
        format!(
            "{}:{}",
            path.file_name().unwrap().to_string_lossy(),
            self.line(pos)
        )
    }

    fn pos_href(&self, pos: Pos) -> String {
        format!(
            "{}#{}",
            self.link(&self.file_path(pos.file)),
            self.line_url_fragment(self.line(pos))
        )
    }

    fn line_url_fragment(&self, n: usize) -> String {
        format!("L{n}")
    }

    fn line(&self, pos: Pos) -> usize {
        self.prog
            .files
            .file_line_map(pos.file)
            .unwrap()
            .line(pos.offset)
    }

    fn types_dir(&self) -> PathBuf {
        PathBuf::from("type")
    }

    fn terms_dir(&self) -> PathBuf {
        PathBuf::from("term")
    }

    fn rules_dir(&self) -> PathBuf {
        PathBuf::from("rule")
    }

    fn expansions_dir(&self) -> PathBuf {
        PathBuf::from("expansion")
    }

    fn expansion_dir(&self, id: usize) -> PathBuf {
        self.expansions_dir().join(id.to_string())
    }

    fn expansion_path(&self, id: usize) -> PathBuf {
        self.expansion_dir(id).join("index.html")
    }

    fn expansion_graph_dot_path(&self, id: usize) -> PathBuf {
        self.expansion_dir(id).join("graph.dot")
    }

    fn expansion_graph_svg_path(&self, id: usize) -> PathBuf {
        self.expansion_dir(id).join("graph.svg")
    }

    fn file_dir(&self) -> PathBuf {
        PathBuf::from("file")
    }

    fn file_path(&self, id: usize) -> PathBuf {
        self.file_dir().join(format!("{id}.html"))
    }

    fn assets_dir(&self) -> PathBuf {
        PathBuf::from("assets")
    }

    fn asset_path(&self, name: &str) -> PathBuf {
        self.assets_dir().join(name)
    }

    fn style_path(&self) -> PathBuf {
        self.asset_path("style.css")
    }

    fn abs(&self, path: &Path) -> PathBuf {
        self.root.join(path)
    }

    fn link(&self, path: &Path) -> String {
        assert!(path.is_relative());
        assert!(self.base.is_relative());

        let mut comps = Vec::new();
        for _ in self.base.components() {
            comps.push(Component::ParentDir);
        }
        comps.extend(path.components());
        let rel: PathBuf = comps.iter().map(|c| c.as_os_str()).collect();
        rel.display().to_string()
    }

    fn create(&mut self, path: &Path) -> io::Result<File> {
        // Path expected to be relative to site root.
        assert!(path.is_relative());

        // Update base directory for relative links.
        self.base = path.parent().expect("should have parent path").into();

        // Create the file, and any parent directories if necessary.
        log::info!("create: {}", path.display());
        let path = self.abs(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        File::create(&path)
    }
}
