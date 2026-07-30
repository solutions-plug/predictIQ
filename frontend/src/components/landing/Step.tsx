import React from 'react';

export interface StepProps {
  title: string;
  description: string;
}

export const Step: React.FC<StepProps> = ({ title, description }) => (
  <li>
    <h3>{title}</h3>
    <p>{description}</p>
  </li>
);
