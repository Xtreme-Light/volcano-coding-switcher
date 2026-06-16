import { colorFor, severityOf } from "../utils/fmt";

interface Props {
  ratio: number; // 0~1
  size?: number;
  thickness?: number;
  label?: string;
}

export default function UsageRing({
  ratio,
  size = 96,
  thickness = 10,
  label,
}: Props) {
  const r = Math.max(0, Math.min(1, ratio));
  const radius = (size - thickness) / 2;
  const circumference = 2 * Math.PI * radius;
  const dash = r * circumference;
  const sev = severityOf(r);
  const color = colorFor(sev);
  const center = size / 2;
  const pct = (r * 100).toFixed(r >= 0.1 ? 0 : 1);

  return (
    <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
      <circle
        cx={center}
        cy={center}
        r={radius}
        fill="none"
        stroke="#2a313a"
        strokeWidth={thickness}
      />
      <circle
        cx={center}
        cy={center}
        r={radius}
        fill="none"
        stroke={color}
        strokeWidth={thickness}
        strokeLinecap="round"
        strokeDasharray={`${dash} ${circumference - dash}`}
        transform={`rotate(-90 ${center} ${center})`}
      />
      <text
        x="50%"
        y="50%"
        textAnchor="middle"
        dy={label ? "-0.1em" : "0.35em"}
        fontSize={size * 0.26}
        fontWeight={600}
        fill={color}
      >
        {pct}%
      </text>
      {label ? (
        <text
          x="50%"
          y="50%"
          textAnchor="middle"
          dy="1.2em"
          fontSize={size * 0.14}
          fill="#8b949e"
        >
          {label}
        </text>
      ) : null}
    </svg>
  );
}
