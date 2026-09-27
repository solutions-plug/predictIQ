'use client';

import React from 'react';
import { SUPPORTED_ASSETS, type SupportedAsset } from '../../lib/assets';
import { useI18n } from '../../lib/hooks/useI18n';
import './TokenSelector.css';

interface TokenSelectorProps {
  id: string;
  value: string;
  onChange: (assetId: string) => void;
  'aria-invalid'?: boolean;
  'aria-describedby'?: string;
}

/**
 * Searchable/selectable list of settlement assets (both Soroban tokens and
 * classic Stellar assets). Selection is restricted to `SUPPORTED_ASSETS` —
 * there is no free-text path to submit an unsupported or malformed asset id.
 */
export function TokenSelector({ id, value, onChange, ...aria }: TokenSelectorProps) {
  const { t } = useI18n();
  const [query, setQuery] = React.useState('');
  const [open, setOpen] = React.useState(false);
  const [activeIndex, setActiveIndex] = React.useState(-1);
  const containerRef = React.useRef<HTMLDivElement>(null);
  const inputRef = React.useRef<HTMLInputElement>(null);

  const selected = SUPPORTED_ASSETS.find((asset) => asset.id === value);

  const filtered = React.useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return SUPPORTED_ASSETS;
    return SUPPORTED_ASSETS.filter(
      (asset) => asset.code.toLowerCase().includes(q) || asset.label.toLowerCase().includes(q)
    );
  }, [query]);

  React.useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setOpen(false);
        setActiveIndex(-1);
      }
    }
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  React.useEffect(() => {
    if (!open) {
      setActiveIndex(-1);
    }
  }, [open]);

  React.useEffect(() => {
    setActiveIndex(-1);
  }, [filtered.length]);

  const selectAsset = (asset: SupportedAsset) => {
    onChange(asset.id);
    setQuery('');
    setOpen(false);
    setActiveIndex(-1);
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (!open && (e.key === 'ArrowUp' || e.key === 'ArrowDown')) {
      e.preventDefault();
      setOpen(true);
      return;
    }

    if (!open) return;

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        setActiveIndex((prev) => (prev < filtered.length - 1 ? prev + 1 : prev));
        break;
      case 'ArrowUp':
        e.preventDefault();
        setActiveIndex((prev) => (prev > 0 ? prev - 1 : -1));
        break;
      case 'Enter':
        e.preventDefault();
        if (activeIndex >= 0 && filtered[activeIndex]) {
          selectAsset(filtered[activeIndex]);
        }
        break;
      case 'Escape':
        e.preventDefault();
        setOpen(false);
        setActiveIndex(-1);
        inputRef.current?.focus();
        break;
    }
  };

  return (
    <div className="token-selector" ref={containerRef}>
      <input
        ref={inputRef}
        id={id}
        type="text"
        role="combobox"
        aria-expanded={open}
        aria-controls={`${id}-listbox`}
        aria-autocomplete="list"
        aria-activedescendant={open && activeIndex >= 0 ? `${id}-option-${activeIndex}` : ''}
        autoComplete="off"
        value={open ? query : selected?.label ?? ''}
        placeholder={t('tokenSelector.searchPlaceholder')}
        onFocus={() => setOpen(true)}
        onChange={(e) => {
          setQuery(e.target.value);
          setOpen(true);
          setActiveIndex(-1);
        }}
        onKeyDown={handleKeyDown}
        {...aria}
      />
      {open && (
        <ul id={`${id}-listbox`} role="listbox" className="token-selector__list">
          {filtered.length === 0 && <li className="token-selector__empty">{t('tokenSelector.noMatching')}</li>}
          {filtered.map((asset, index) => (
            <li key={asset.id}>
              <button
                id={`${id}-option-${index}`}
                type="button"
                role="option"
                aria-selected={asset.id === value}
                className={`token-selector__option ${activeIndex === index ? 'token-selector__option--active' : ''}`}
                onClick={() => selectAsset(asset)}
              >
                <span className="token-selector__code">{asset.code}</span>
                <span className="token-selector__label">{asset.label}</span>
                <span className="token-selector__kind">
                  {asset.kind === 'soroban_token' ? t('tokenSelector.sorobanToken') : t('tokenSelector.classicAsset')}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
