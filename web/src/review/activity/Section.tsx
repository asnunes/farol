/** A heading inside the box's list: In progress, Updates. */
export function Section({ children }: SectionProps) {
  return (
    <li className="px-2.5 pt-2 pb-0.5 font-sans text-[0.6875rem] tracking-wider text-faint uppercase">
      {children}
    </li>
  );
}

type SectionProps = { children: React.ReactNode };
