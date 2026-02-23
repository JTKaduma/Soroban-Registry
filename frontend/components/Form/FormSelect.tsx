import React from 'react';
import FormField from './FormField';

type Option = { value: string | number; label: string };

type Props = React.SelectHTMLAttributes<HTMLSelectElement> & {
  label?: string;
  options: Option[];
  error?: string;
  description?: string;
};

export default function FormSelect({ label, options, error, description, className, ...rest }: Props) {
  const id = rest.id || rest.name;
  return (
    <FormField label={label} id={id} error={error} description={description}>
      <select
        {...rest}
        id={id}
        className={
          'w-full px-3 py-2 rounded-lg border border-border bg-background text-foreground focus:outline-none focus:ring-2 focus:ring-primary/20 ' +
          (className || '')
        }
        aria-invalid={!!error}
      >
        {options.map((opt) => (
          <option key={String(opt.value)} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>
    </FormField>
  );
}
