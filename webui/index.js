
    let apiBaseUrl = 'http://127.0.0.1:5010';
    let isPlaying = false;
    let isRepeating = false;
    let currentSongUUID = null;

    if (!window.webkit || !window.webkit.messageHandlers) {
        var storedApiBaseUrl = localStorage.getItem('apiBaseUrl');
        if (storedApiBaseUrl) {
            apiBaseUrl = storedApiBaseUrl;
        }
    }

    function saveApiBaseUrl(apiBaseUrl) {
        if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.saveApiBaseUrl) {
            window.webkit.messageHandlers.saveApiBaseUrl.postMessage(apiBaseUrl);
        } else {
            // Fallback for web server environment
            console.log("Saving apiBaseUrl:", apiBaseUrl);
            localStorage.setItem('apiBaseUrl', apiBaseUrl);
        }
    }
    

    function navigateTo(url) {
        const iframe = document.getElementById('content-frame');
        const newUrl = `${url}&apibaseUrl=${encodeURIComponent(apiBaseUrl)}`;
        iframe.src = newUrl;
    }

    function navigateToForLikedAndAll(url) {
        const iframe = document.getElementById('content-frame');
        const newUrl = `${url}?apibaseUrl=${encodeURIComponent(apiBaseUrl)}`;
        iframe.src = newUrl;
    }

    function handleSearchKey(event) {
        if (event.key === 'Enter') {
            navigateToSearch();
        }
    }

    function navigateToSearch() {
        const query = document.getElementById('search-query').value;
        console.log(`Navigating to search.html?query=${encodeURIComponent(query)}`); // Log URL for debugging
        navigateTo(`search.html?query=${encodeURIComponent(query)}`);
    }

    function setapiBaseUrl() {
        const query = document.getElementById('apiBaseUrl').value;
        if (query) {
            console.log(`set apibaseUrl to ${encodeURIComponent(query)}`); // Log URL for debugging
            apiBaseUrl=query;
        } else {
            alert('请输入apibaseurl');
        }
        saveApiBaseUrl(apiBaseUrl);
    }

    function navigateToLiked() {
        console.log('Navigating to liked.html'); // Log navigation for debugging
        navigateToForLikedAndAll('liked.html');
    }

    function navigateToAll() {
        console.log('Navigating to all.html'); // Log navigation for debugging
        navigateToForLikedAndAll('all.html');
    }

    // 播放器控制逻辑
    const audioPlayer = document.getElementById('audio-player');

    if ('mediaSession' in navigator) {
        function updateMediaSessionMetadata(song) {
            navigator.mediaSession.metadata = new MediaMetadata({
                title: song.title,
                artist: song.artists.map(artist => artist.name).join(', '),
                album: song.album,
                artwork: [
                    { src: song.art || 'default_cover.png', sizes: '96x96', type: 'image/png' },
                    { src: song.art || 'default_cover.png', sizes: '128x128', type: 'image/png' },
                    { src: song.art || 'default_cover.png', sizes: '192x192', type: 'image/png' },
                    { src: song.art || 'default_cover.png', sizes: '256x256', type: 'image/png' },
                    { src: song.art || 'default_cover.png', sizes: '384x384', type: 'image/png' },
                    { src: song.art || 'default_cover.png', sizes: '512x512', type: 'image/png' },
                ]
            });

            // 设置媒体会话操作处理程序
            navigator.mediaSession.setActionHandler('play', togglePlayPause);
            navigator.mediaSession.setActionHandler('pause', togglePlayPause);
            navigator.mediaSession.setActionHandler('previoustrack', playPreviousSong);
            navigator.mediaSession.setActionHandler('nexttrack', playNextSong);
        }
    }

    async function playSong(song) {
        try {
            console.log('Playing song:', song);

            // 暂停并重置当前音频
            audioPlayer.pause();
            audioPlayer.src = ''; // 清空当前 src
            audioPlayer.load(); // 重置音频元素

            const streamUrl = `${apiBaseUrl}/getStream?file_path=${encodeURIComponent(song.filePath)}`;
            audioPlayer.src = streamUrl;
            console.log(streamUrl);
            audioPlayer.play();
            isPlaying = true;
            updatePlayPauseButton();

            document.getElementById('playing-art').src = song.art || 'default_cover.png';
            document.getElementById('playing-title').textContent = song.title;

            const artistsHtml = song.artists.map(artist => `<a href="#" onclick="navigateToArtistPage('${artist.uuid}')" style="color: #007bff; margin-right: 5px;">${artist.name}</a>`).join('');
            document.getElementById('playing-artists').innerHTML = artistsHtml;
            document.getElementById('playing-album').innerHTML = `<a href="#" onclick="navigateToAlbumPage('${song.albumUUID}')" style="color: #007bff; margin-left: 10px;">${song.album}</a>`;

            updateLikeButton(song.isLiked);

            // 更新媒体会话元数据
            if ('mediaSession' in navigator) {
                updateMediaSessionMetadata(song);
            }

            audioPlayer.onloadedmetadata = () => {
                document.getElementById('song-duration').textContent = formatTime(audioPlayer.duration);
            };

            audioPlayer.ontimeupdate = () => {
                document.getElementById('playing-time').textContent = formatTime(audioPlayer.currentTime);
                const progressPercent = (audioPlayer.currentTime / audioPlayer.duration) * 100;
                document.getElementById('progress').style.width = `${progressPercent}%`;
            };

            audioPlayer.onended = () => {
                if (isRepeating) {
                    audioPlayer.currentTime = 0;
                    audioPlayer.play();
                } else {
                    playNextSong();
                }
            };

            currentSongUUID = song.uuid;
        } catch (error) {
            console.error('Error:', error);
        }
    }

    function playPreviousSong() {
        const iframe = document.getElementById('content-frame').contentWindow;
        iframe.postMessage({ action: 'previous' }, '*');
    }

    function playNextSong() {
        const iframe = document.getElementById('content-frame').contentWindow;
        console.log("next song message sent!")
        iframe.postMessage({ action: 'next' }, '*');
    }
    function navigateToArtistPage(artistUUID) {
        navigateTo(`artist.html?uuid=${artistUUID}`);
    }

    function navigateToAlbumPage(albumUUID) {
        navigateTo(`album.html?uuid=${albumUUID}`);
    }

    function updatePlayPauseButton() {
        const playPauseBtn = document.getElementById('play-pause-btn');
        playPauseBtn.innerHTML = isPlaying ? '&#10074;&#10074;' : '&#9654;';
    }

    function togglePlayPause() {
        if (isPlaying) {
            audioPlayer.pause();
            isPlaying = false;
        } else {
            audioPlayer.play();
            isPlaying = true;
        }
        updatePlayPauseButton();
    }

    function toggleRepeat() {
        isRepeating = !isRepeating;
        const repeatBtn = document.getElementById('repeat-btn');
        repeatBtn.classList.toggle('active'); // Toggle active class for visual feedback
        repeatBtn.innerHTML = isRepeating ? '&#128258;' : '&#128257;';
    }

    function formatTime(seconds) {
        const minutes = Math.floor(seconds / 60);
        const secs = Math.floor(seconds % 60);
        return `${minutes}:${secs < 10 ? '0' : ''}${secs}`;
    }

    function setProgress(e) {
        const progressBar = document.getElementById('progress-bar');
        const width = progressBar.clientWidth;
        const clickX = e.offsetX;
        const duration = audioPlayer.duration;

        audioPlayer.currentTime = (clickX / width) * duration;
    }

    function updateLikeButton(isLiked) {
        const likeBtn = document.getElementById('like-btn');
        if (isLiked) {
            likeBtn.innerHTML = '&#9829;';
            likeBtn.classList.add('text-danger');
        } else {
            likeBtn.innerHTML = '&#9825;';
            likeBtn.classList.remove('text-danger');
        }
    }

    async function toggleSongLike() {
        if (!currentSongUUID) {
            console.error('Error: No song is currently being played.');
            return;
        }

        try {
            const response = await fetch(`${apiBaseUrl}/show_song/${currentSongUUID}`);
            if (response.ok) {
                const song = await response.json();
                let likeResponse;
                if (song.is_liked) {
                    likeResponse = await fetch(`${apiBaseUrl}/unlike_song/${currentSongUUID}`);
                } else {
                    likeResponse = await fetch(`${apiBaseUrl}/like_song/${currentSongUUID}`);
                }

                if (likeResponse.ok) {
                    const updatedSongResponse = await fetch(`${apiBaseUrl}/show_song/${currentSongUUID}`);
                    if (updatedSongResponse.ok) {
                        const updatedSong = await updatedSongResponse.json();
                        updateLikeButton(updatedSong.is_liked);
                        // 发送消息给 iframe 更新列表中的爱心图标
                        const iframe = document.getElementById('content-frame').contentWindow;
                        iframe.postMessage({ action: 'updateSongLike', uuid: currentSongUUID, isLiked: updatedSong.is_liked }, '*');
                    }
                } else {
                    console.error('Failed to toggle like status:', likeResponse.statusText);
                }
            } else {
                console.error('Failed to fetch song details:', response.statusText);
            }
        } catch (error) {
            console.error('Error:', error);
        }
    }
    function playNextSong() {
        const iframe = document.getElementById('content-frame').contentWindow;
        iframe.postMessage({ action: 'next' }, '*');
    }

    document.getElementById('prev-btn').addEventListener('click', () => {
        const iframe = document.getElementById('content-frame').contentWindow;
        iframe.postMessage({ action: 'previous' }, '*');
    });

    document.getElementById('play-pause-btn').addEventListener('click', togglePlayPause);
    document.getElementById('prev-btn').addEventListener('click', playPreviousSong);
    document.getElementById('next-btn').addEventListener('click', playNextSong);
    document.getElementById('repeat-btn').addEventListener('click', toggleRepeat);
    document.getElementById('like-btn').addEventListener('click', toggleSongLike);
    document.getElementById('progress-bar').addEventListener('click', setProgress);

    // 接收来自 iframe 的消息
    window.addEventListener('message', (event) => {
        if (event.data.action === 'playSong') {
            playSong(event.data.song);
        } else if (event.data.action === 'navigateTo') {
            navigateTo(event.data.url);
        }
    });

