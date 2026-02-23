import React from 'react';
import FormField from './FormField';

type Props = React.InputHTMLAttributes<HTMLInputElement> & {
  label?: string;
  error?: string;
  description?: string;
};

export default function FormInput({ label, error, description, className, ...rest }: Props) {
  const id = rest.id || rest.name;
  return (
    <FormField label={label} id={id} error={error} description={description}>
      <input
        {...rest}
        id={id}
        className={
          'w-full px-3 py-2 rounded-lg border border-border bg-background text-foreground focus:outline-none focus:ring-2 focus:ring-primary/20 ' +
          (className || '')
        }
        aria-invalid={!!error}
        aria-describedby={error ? `${id}-error` : undefined}
      />
    </FormField>
  );
}
