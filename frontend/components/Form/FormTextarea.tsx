import React from 'react';
import FormField from './FormField';

type Props = React.TextareaHTMLAttributes<HTMLTextAreaElement> & {
  label?: string;
  error?: string;
  description?: string;
};

export default function FormTextarea({ label, error, description, className, ...rest }: Props) {
  const id = rest.id || rest.name;
  return (
    <FormField label={label} id={id} error={error} description={description}>
      <textarea
        {...rest}
        id={id}
        className={
          'w-full px-3 py-2 rounded-lg border border-border bg-background text-foreground focus:outline-none focus:ring-2 focus:ring-primary/20 ' +
          (className || '')
        }
        aria-invalid={!!error}
      />
    </FormField>
  );
}
