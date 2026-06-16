interface Props {
  lines: string[];
}

export default function EventLog({ lines }: Props) {
  return (
    <section className="card">
      <h2 className="text-base font-semibold mb-2">事件日志</h2>
      <pre className="bg-panel2 border border-border rounded-md p-3 text-xs leading-relaxed max-h-72 overflow-auto whitespace-pre-wrap">
        {lines.length === 0 ? "(尚无事件)" : lines.join("\n")}
      </pre>
    </section>
  );
}
