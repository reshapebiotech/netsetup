/** @type {import('tailwindcss').Config}*/

// made color tokens to reference them in the semantic tokens
const colors = {
  shade: {
    50: '#A0A0A0',
    100: '#F1F1F1',
    200: '#E3E3E3',
    700: '#686868',
  },
  primary: {
    100: 'rgba(87, 80, 238, 0.1)',
    500: '#5750EE',
  },
  background: {
    500: '#F9F9FA', // mist
  },
  shadow: {
    2: 'rgba(0, 0, 0, 0.02)',
    12: 'rgba(0, 0, 0, 0.12)',
    14: 'rgba(18, 18, 18, 0.14)',
    40: 'rgba(18, 18, 18, 0.04)',
    160: 'rgba(18, 18, 18, 0.16)',
  },
  midnight: {
    10: 'rgba(18, 25, 42, 0.10)',
  },
  dark: {
    5: 'rgba(18, 25, 42, 0.05)',
    10: 'rgba(18, 25, 42, 0.1)',
    60: 'rgb(18, 25, 42, 0.6)',
    70: 'rgb(18, 25, 42, 0.7)',
    500: '#12192A', // navy-navy
  },
  light: {
    500: '#F6F7F7', // off-white
  },
  white: {
    500: '#FFF',
    hover: '#f0f0f0',
  },
  yellow: {
    300: '#FEBE00',
    500: '#F0C321',
  },
  orange: {
    500: '#FF7A00',
  },
  red: {
    100: '#FFEDEC',
    300: '#F14400',
    500: '#C13500',
  },
  green: {
    300: '#65cba4',
    500: '#0aad6e',
  },
  black: {
    500: '#000',
  },
  gray: {
    // navy
    51: 'rgba(18, 25, 42, 0.05)', // 51 is a workaround, using 50 causes tailwind to use a different color
    100: 'rgba(18, 25, 42, 0.10)',
    200: 'rgba(18, 25, 42, 0.20)',
    300: 'rgba(18, 25, 42, 0.30)',
    400: 'rgba(18, 25, 42, 0.40)',
    500: 'rgba(18, 25, 42, 0.50)',
    600: 'rgba(18, 25, 42, 0.60)',
    700: 'rgba(18, 25, 42, 0.70)',
  },
  base: {
    100: '#F3F9FA', // teal-light
  },
  sky: {
    200: '#23EBFF',
    500: '#70D4FF',
  },
  periwinkle: {
    1: '#eeeefd',
    2: '#cbc9fa',
    3: '#8e8af4',
    4: '#7973f1',
    5: '#5750ee',
    6: '#4741c1',
  },
};

const config = {
  content: ['./src/**/*.{html,js,svelte,ts}', './src/tailwind-safelist.txt'],

  theme: {
    extend: {
      colors,
      borderWidth: {
        1: '1px',
      },
      borderColor: {
        DEFAULT: 'rgba(18, 25, 42, 0.06)',
      },
      boxShadow: {
        DEFAULT: '0 2px 16px 0 rgba(18, 25, 42, 0.02), 0 0 0 1px rgba(18, 25, 42, 0.06)',
        hover: '0 2px 16px 0 rgba(18, 25, 42, 0.02), 0 0 0 1px rgba(87, 80, 238, 0.25)',
        sm: '0 4px 17px 0 rgba(18, 25, 42, 0.02)',
      },
      fontFamily: {
        mono: ['"Geist Mono"', 'monospace'],
        sans: ['"af Another Sans"', 'sans-serif'],
      },
    },
  },

  plugins: [require('daisyui')],
  daisyui: {
    base: true,
    styled: true,
    logs: false,
    themes: [
      {
        light: {
          primary: '#5750EE',
          secondary: '#E3E3E3',
          accent: '#E3E3E3',
          neutral: '#A0A0A0',
          info: '#23EBFF',
          success: '#42E55C',
          warning: '#FEBE00',
          error: '#F14400',
          default: '#5750EE',
        },
      },
    ],
  },
};

module.exports = config;
