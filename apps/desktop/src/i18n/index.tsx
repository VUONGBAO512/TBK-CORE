import React, { createContext, useContext, useState } from 'react';
import { en } from './en';
import { vi } from './vi';

export type Language = 'en' | 'vi';
export type Translations = typeof en;

interface I18nContextType {
  lang: Language;
  setLang: (lang: Language) => void;
  t: Translations;
}

const I18nContext = createContext<I18nContextType>({
  lang: 'vi',
  setLang: () => {},
  t: vi,
});

export const I18nProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [lang, setLangState] = useState<Language>(() => {
    const saved = localStorage.getItem('pc_doctor_lang');
    return saved === 'en' ? 'en' : 'vi';
  });

  const setLang = (newLang: Language) => {
    setLangState(newLang);
    localStorage.setItem('pc_doctor_lang', newLang);
  };

  const t = lang === 'en' ? en : vi;

  return (
    <I18nContext.Provider value={{ lang, setLang, t }}>
      {children}
    </I18nContext.Provider>
  );
};

export const useI18n = () => useContext(I18nContext);
