let theme = $state({
    theme: localStorage.getItem('theme') || 'dark'
});

export function toggleTheme() {
    theme.theme = theme.theme === 'dark' ? 'light' : 'dark';

    localStorage.setItem('theme', theme.theme);

    Object.entries(themes[theme.theme]).forEach(([key, value]) => {
        document.documentElement.style.setProperty(key, value as string);
    });
}

//called on mount by the root layout component
export function loadTheme() {
    Object.entries(themes[theme.theme]).forEach(([key, value]) => {
        document.documentElement.style.setProperty(key, value as string);
    });
}

export const themes: any = {
    light: {
        "--background-color": "#ffffff",
        "--text-color": "#000000",
        "--border-color": "#000000",
        "--button-background-color": "rgba(200, 200, 200, 0.2)",
        "--button-hover-color": "rgba(200, 200, 200, 0.5)",
        "--header-background-color": "rgba(200, 200, 200, 0.1)",
        "--sidebar-color": "rgba(200, 200, 200, 0.05)",
        "--black-image-filter": "invert(0)",
        
    },
    dark: {
        "--header-background-color": "rgba(255, 255, 255, 0.1)",
        "--border-color": "#ffffff",
        "--button-background-color": "rgba(255, 255, 255, 0.1)",
        "--button-hover-color": "rgba(255, 255, 255, 0.2)",
        "--text-color": "#ffffff",
        "--background-color": "rgb(55, 55, 55)",
        "--sidebar-color": "rgba(255, 255, 255, 0.0.5);",
        "--black-image-filter": "invert(1)",
    }
}

export default theme;