<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { open } from '@tauri-apps/plugin-dialog';
    import { listen, type UnlistenFn } from '@tauri-apps/api/event';
    import { onMount } from "svelte";
    import Swal from 'sweetalert2'

    interface Student {
        sid: string;
        text: string;
        submissionId: string;
    }

    interface Listeners {
        stdout: Promise<UnlistenFn>;
        stderr: Promise<UnlistenFn>;
    }

    let lsData: string[] = $state([]);
    // let dirName: string = $state("");
    let port: string = $state('4131');
    let portEnabled = $state(false);
    let cdData: string = $state("");
    let lsDirData: string[] = $state([]);
    let studentSubmissions: any[] = $state([]);
    let terminalText: string = $state("");
    let currentStudent: Student | null = $state(null);
    let listeners: Listeners | null = $state(null);
    let showDirsWithStudents = $state(false);
    let currentPid = 0;

    async function ls() {
        if (lsData.length) {
            lsData = [];
            return;
        }
        lsData = (await invoke("ls") as string).split("\n");
    }

    async function pwd() {
        cdData = await invoke("pwd");
    }

    async function ls_directories() {
        lsDirData = (await invoke("ls_directories") as string).split("\n");
        lsDirData.unshift("..");

        getAllSubmitters();
    }

    function toggleDirs() {
        if (lsDirData.length > 0) {
            lsDirData = [];
            studentSubmissions = [];
        } else {
            ls_directories();
        }
    }

    async function openDirectory() {
        const dir = await open({
            directory: true,
            multiple: false,
        });
        
        if (dir) {
            cd(dir);
        }
    }

    async function cd(name: string) {
        cdData = await invoke("cd", { name });
        // lsData = (await invoke("ls") as string).split("\n");
        if (lsDirData.length > 0) {
            ls_directories();
        }
    }

    async function handleDirClick(dir: string) {
        console.log(dir);
        cd(dir);
    }

    async function getAllSubmitters() {
        let submissionDir = await invoke("read_submission_dir") as string;
        let submissionJSON: any;
        try {
            submissionJSON = JSON.parse(submissionDir);
        }
        catch {
            // error from JSON parsing an error string, there is no YAML file in here
            studentSubmissions = [];
            return;
        }

        //reset student submissions before looping over each and adding it
        studentSubmissions = [];
        Object.keys(submissionJSON).forEach((key) => {
            try {
                const studentData = submissionJSON[key].submitters[0];
                const submissionId = key.split('_')[1];
                const obj: Student = {
                    text: `${studentData.name} (${submissionId})`,
                    sid: studentData.sid,
                    submissionId
                }
                studentSubmissions.push(obj);
            }
            catch {
                console.log("error parsing student data");
                return;
            }
        })

        //sort all submissions by last name
        studentSubmissions.sort((a, b) => {
            const textA = a.text.split(' ')[1];
            const textB = b.text.split(' ')[1];
            return textA.localeCompare(textB);
        });
    }

    async function handleStudentClick(student: Student) {
        let realPort = parseInt(port);
        if (isNaN(realPort) || realPort < 1 || realPort > 65535) {
            console.log("invalid port: " + port);
            Toast.fire({
                icon: 'error',
                text: 'Invalid port number'
            });
            return;
        }

        if (currentPid) {
            await killServer(currentPid);
        }

        //construct arguments and start the server
        const args: any = {submissionId: student.submissionId };
        args.port = portEnabled ? realPort : 0;
        const data = await invoke("handle_student_click", args);

        if (!parseInt(data as string)) {
            console.log("error starting server: " + data);
            Toast.fire({
                icon: 'error',
                title: "Server failed to start",
                text: data as string
            });
            return;
        }

        currentStudent = student;
        currentPid = parseInt(data as string);
        console.log("server started with pid: " + currentPid);

        //unlisten from previous listeners
        if (listeners) {
            (await listeners.stdout)();
            (await listeners.stderr)();
        }

        listeners = {
            stdout: listen("server-output", (event) => {
                terminalText += event.payload + "\n";
            }),
            stderr: listen("server-error", (event) => {
                if (!terminalText) {

                }
                terminalText += event.payload + "\n";
            })
        }
    }

    async function killServer(pid: number) {
        await invoke("kill_server", {pid});
        currentStudent = null;
        currentPid = 0;
        terminalText = "";
    }


    const Toast = Swal.mixin({
        toast: true,
        position: 'bottom-end',
        iconColor: 'white',
        customClass: {
            popup: 'colored-toast',
        },
        showClass: {
            popup: '',
        },
        hideClass: {
            popup: '',
        },
        showConfirmButton: false,
        timer: 2000,
        timerProgressBar: true,
    });

    //get the working directory on mount
    onMount(() => {
        pwd();
    });
</script>

<svelte:head>
    <link rel="stylesheet" href="/style/file-list.css">
</svelte:head>

<main>
    {#if studentSubmissions.length > 0}
        <div class="main-left">
            <div class="title">
                student submissions
            </div>
            {#each studentSubmissions as student}
                <div class="left-dir-entry" onclick={() => handleStudentClick(student)}>
                    <div class="inner">
                        {student.text.split('(')[0]}
                        <br>
                        <small>
                            {'(' + student.text.split('(')[1]}
                        </small>
                    </div>
                </div>
            {/each}
        </div>
    {/if}
    <div class="main-right">
        <div class="flex-center">
            <small>
                ({cdData})
            </small>
            <div>
                <button onclick={openDirectory}>
                    open folder
                </button>
                <button onclick={toggleDirs}>
                    {lsDirData.length > 0 ? "hide" : "list"}
                    directories
                </button>
                <button onclick={ls}>
                    {lsData.length > 0 ? "hide" : "list"}
                    files
                </button>
            </div>
            <!-- <div>
                <input type="text" bind:value={dirName}>
                <button onclick={() => cd(dirName)}>change directory</button>
            </div> -->
            <div>
                <input type="text" bind:value={port} style="{portEnabled ? '' : 'border: 1px solid gray; color: gray'}" disabled={!portEnabled}>
                <button onclick={() => portEnabled = !portEnabled}>
                    {portEnabled ? "disable" : "enable"} port argument
                </button>
            </div>
        </div>
        <div class="flex-center">
            {#if lsDirData.length > 0 && studentSubmissions.length == 0}
                <div style="font-weight: 600;">
                    directories
                </div>
                <div class="dir-list">
                    {#each lsDirData as dir}
                        {#if dir}
                            <div class="dir-entry" onclick={() => handleDirClick(dir)}>
                                {dir}
                            </div>
                        {/if}
                    {/each}
                </div>
            {/if}
            {#if studentSubmissions.length != 0}
                <button onclick={() => showDirsWithStudents = !showDirsWithStudents}>
                    {showDirsWithStudents ? "hide" : "show"} all directories
                </button>

                {#if showDirsWithStudents}
                    <div style="font-weight: 600;">
                        directories
                    </div>
                    <div class="dir-list">
                        {#each lsDirData as dir}
                            {#if dir}
                                <div class="dir-entry" onclick={() => handleDirClick(dir)}>
                                    {dir}
                                </div>
                            {/if}
                        {/each}
                    </div>
                {/if}
            {/if}
        </div>
        <div class="flex-center">
            {#if lsData.length > 0}
                <div style="font-weight: 600;">
                    files
                </div>
                {#each lsData as file}
                    {file}
                    <br>
                {/each}
            {/if}
        </div>
        {#if currentStudent}
            <div class="flex-center">
                <div>
                    currently running: {currentStudent.text.split('(')[0]}
                </div>
                <button onclick={() => killServer(currentPid)}>
                    kill server
                </button>
            </div>
            <div class="flex-center">
                <div style="font-weight: 600;">
                    terminal output
                </div>
                <div class="terminal">
                    {#each terminalText.split('\n') as line}
                        {line}
                        <br>
                    {/each}
                </div>
            </div>
        {/if}
        <br>
    </div>
</main>