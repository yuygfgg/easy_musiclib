
    let apiBaseUrl = 'http://127.0.0.1:5010';
    const urlParams = new URLSearchParams(window.location.search);
    const albumUUID = urlParams.get('uuid');
    apiBaseUrl = urlParams.get('apibaseUrl');

    if (!albumUUID) {
        document.body.innerHTML = '<h1>缺少专辑UUID参数</h1>';
        throw new Error('缺少专辑UUID参数');
    }

    let currentIndex = 0;
    let songs = [];
    let currentSong = null;

    async function fetchAlbumData() {
        try {
            const response = await fetch(`${apiBaseUrl}/show_album/${albumUUID}`);
            if (response.ok) {
                const albumData = await response.json();
                console.log('Fetched album data:', albumData);
                songs = albumData.songs;
                displayAlbumData(albumData);
            } else {
                console.error('Failed to fetch album data:', response.statusText);
            }
        } catch (error) {
            console.error('Error:', error);
        }
    }

    async function fetchFile(filePath) {
        try {
            const response = await fetch(`${apiBaseUrl}/getfile?file_path=${encodeURIComponent(filePath)}`);
            if (response.ok) {
                return response.blob();
            } else {
                console.error('Failed to fetch file:', response.statusText);
            }
        } catch (error) {
            console.error('Error:', error);
        }
        return null;
    }

    async function displayAlbumData(album) {
        const albumArtBlob = await fetchFile(album.album_art_path);
        if (albumArtBlob) {
            const albumArtUrl = URL.createObjectURL(albumArtBlob);
            document.getElementById('album-art').src = albumArtUrl;
        } else {
            document.getElementById('album-art').src = 'default_cover.png';
        }

        document.getElementById('album-name').textContent = album.name;
        document.getElementById('album-year').textContent = `年代: ${album.year}`;
        updateAlbumLikeButton(album);

        const albumArtists = document.getElementById('album-artists');
        albumArtists.innerHTML = '';
        album.album_artists.forEach(artist => {
            const artistLink = document.createElement('a');
            artistLink.href = `artist.html?uuid=${artist.uuid}&apibaseUrl=${apiBaseUrl}`;
            artistLink.textContent = artist.name;
            artistLink.style.color = '#007bff';
            artistLink.style.marginRight = '10px';
            albumArtists.appendChild(artistLink);
        });

        document.getElementById('song-count').textContent = album.songs.length;

        const songList = document.getElementById('song-list');
        songList.innerHTML = '';

        album.songs.sort((a, b) => {
            if (a.disc_number === b.disc_number) {
                return a.track_number - b.track_number;
            } else {
                return a.disc_number - b.disc_number;
            }
        });

        let currentDiscNumber = -1;

        for (const [index, song] of album.songs.entries()) {
            if (song.disc_number !== currentDiscNumber) {
                currentDiscNumber = song.disc_number;
                const discSeparator = document.createElement('div');
                discSeparator.className = 'disc-separator';
                discSeparator.textContent = `Disc ${currentDiscNumber}`;
                discSeparator.style.margin = '20px 0';
                discSeparator.style.fontWeight = 'bold';
                songList.appendChild(discSeparator);
            }

            const songItem = document.createElement('div');
            songItem.className = 'song-item';
            songItem.id = `song-${song.uuid}`;
            songItem.onclick = (event) => {
                if (event.target.tagName.toLowerCase() !== 'span' && event.target.tagName.toLowerCase() !== 'a') {
                    fetchAndPlaySong(index);
                }
            };

            const songInfo = document.createElement('div');
            songInfo.className = 'song-info';

            const songNumber = document.createElement('div');
            songNumber.className = 'number';
            songNumber.textContent = index + 1;

            const songArt = document.createElement('img');
            songArt.className = 'song-art';
            const songArtBlob = await fetchFile(song.song_art_path);
            if (songArtBlob) {
                const songArtUrl = URL.createObjectURL(songArtBlob);
                songArt.src = songArtUrl;
            } else {
                songArt.src = 'default_cover.png';
            }
            songArt.alt = song.name;

            const songDetails = document.createElement('div');
            songDetails.className = 'details';

            const songTitle = document.createElement('div');
            songTitle.className = 'title';
            songTitle.textContent = song.name;

            const songArtist = document.createElement('div');
            songArtist.className = 'artist';
            song.artists.forEach(artist => {
                const artistLink = document.createElement('a');
                artistLink.href = '#';
                artistLink.textContent = artist.name;
                artistLink.style.color = '#007bff';
                artistLink.style.marginRight = '5px';
                artistLink.onclick = (event) => {
                    event.stopPropagation();
                    window.parent.postMessage({ action: 'navigateTo', url: `artist.html?uuid=${artist.uuid}` }, '*');
                };
                songArtist.appendChild(artistLink);
            });

            songDetails.appendChild(songTitle);
            songDetails.appendChild(songArtist);
            songInfo.appendChild(songNumber);
            songInfo.appendChild(songArt);
            songInfo.appendChild(songDetails);

            const songActions = document.createElement('div');
            songActions.className = 'actions';

            const likedIcon = document.createElement('span');
            likedIcon.className = song.is_liked ? 'text-danger liked' : 'liked';
            likedIcon.innerHTML = song.is_liked ? '&#9829;' : '&#9825;';
            likedIcon.addEventListener('click', (event) => {
                event.stopPropagation();
                toggleSongLike(song.uuid);
            });
            songActions.appendChild(likedIcon);

            songItem.appendChild(songInfo);
            songItem.appendChild(songActions);
            songList.appendChild(songItem);
        }
        const paddingElement = document.createElement('div');
        paddingElement.style.height = '100px';
        songList.appendChild(paddingElement);
    }

    function updateAlbumLikeButton(album) {
        const albumLike = document.getElementById('album-like');
        albumLike.innerHTML = album.is_liked ? '喜欢 &#9829;' : '不喜欢 &#9825;';
        albumLike.className = album.is_liked ? 'liked' : 'unliked';
        albumLike.onclick = () => toggleAlbumLike(album);
    }

    async function toggleAlbumLike(album) {
        const uuid = album.uuid;
        try {
            let likeResponse;
            if (album.is_liked) {
                likeResponse = await fetch(`${apiBaseUrl}/unlike_album/${uuid}`);
            } else {
                likeResponse = await fetch(`${apiBaseUrl}/like_album/${uuid}`);
            }

            if (likeResponse.ok) {
                const updatedAlbumResponse = await fetch(`${apiBaseUrl}/show_album/${uuid}`);
                if (updatedAlbumResponse.ok) {
                    const updatedAlbum = await updatedAlbumResponse.json();
                    updateAlbumLikeButton(updatedAlbum);
                }
            } else {
                console.error('Failed to toggle like status for album:', likeResponse.statusText);
            }
        } catch (error) {
            console.error('Error:', error);
        }
    }

    async function fetchAndPlaySong(index) {
        currentIndex = index;
        const uuid = songs[index].uuid;
        try {
            console.log('Fetching song details for uuid:', uuid);
            const response = await fetch(`${apiBaseUrl}/show_song/${uuid}`);
            if (response.ok) {
                const song = await response.json();
                console.log('Fetched song details:', song);
                playSong(song);
            } else {
                console.error('Failed to fetch song details:', response.statusText);
            }
        } catch (error) {
            console.error('Error:', error);
        }
    }

    async function playSong(song) {
        try {
            console.log('Playing song:', song);
            if (!song.file_path) {
                console.error('Error: song.file_path is undefined');
                return;
            }

            const songArtBlob = await fetchFile(song.song_art_path);
            let songArtUrl = 'default_cover.png';
            if (songArtBlob) {
                songArtUrl = URL.createObjectURL(songArtBlob);
            }

            window.parent.postMessage({
                action: 'playSong',
                song: {
                    filePath: song.file_path,
                    art: songArtUrl,
                    title: song.name,
                    artists: song.artists.map(artist => ({ name: artist.name, uuid: artist.uuid })),
                    album: song.album.name,
                    albumUUID: song.album.uuid,
                    isLiked: song.is_liked,
                    uuid: song.uuid
                }
            }, '*');

            currentSong = song;
        } catch (error) {
            console.error('Error:', error);
        }
    }

    async function toggleSongLike(uuid) {
        try {
            const response = await fetch(`${apiBaseUrl}/show_song/${uuid}`);
            if (response.ok) {
                const song = await response.json();
                let likeResponse;
                if (song.is_liked) {
                    likeResponse = await fetch(`${apiBaseUrl}/unlike_song/${uuid}`);
                } else {
                    likeResponse = await fetch(`${apiBaseUrl}/like_song/${uuid}`);
                }

                if (likeResponse.ok) {
                    const updatedSongResponse = await fetch(`${apiBaseUrl}/show_song/${uuid}`);
                    if (updatedSongResponse.ok) {
                        const updatedSong = await updatedSongResponse.json();
                        updateSongListIcon(uuid, updatedSong.is_liked);
                        if (currentSong && currentSong.uuid === uuid) {
                            currentSong.is_liked = updatedSong.is_liked;
                        }
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

    async function updateSongListIcon(uuid, isLiked) {
        const songItem = document.getElementById(`song-${uuid}`);
        if (songItem) {
            const likedIcon = songItem.querySelector('.actions span');
            likedIcon.className = isLiked ? 'text-danger liked' : 'liked';
            likedIcon.innerHTML = isLiked ? '&#9829;' : '&#9825;';
        }
    }

    window.addEventListener('message', (event) => {
        if (event.data.action === 'previous') {
            playPreviousSong();
        } else if (event.data.action === 'next') {
            playNextSong();
        } else if (event.data.action === 'toggleLike') {
            toggleLike();
        }
    });

    async function toggleLike() {
        if (!currentSong) {
            console.error('Error: No song is currently being played.');
            return;
        }

        await toggleSongLike(currentSong.uuid);
    }

    let isDebounced = false;

    function playNextSong() {
        if (isDebounced) return;

        isDebounced = true;
        setTimeout(() => { isDebounced = false; }, 1000);

        console.log("next song!");
        currentIndex = (currentIndex + 1) % songs.length;
        fetchAndPlaySong(currentIndex);
    }

    function playPreviousSong() {
        if (isDebounced) return;

        isDebounced = true;
        setTimeout(() => { isDebounced = false; }, 1000);

        currentIndex = (currentIndex - 1 + songs.length) % songs.length;
        fetchAndPlaySong(currentIndex);
    }

    fetchAlbumData();
    window.addEventListener('message', (event) => {
        if (event.data.action === 'previous') {
            playPreviousSong();
        } else if (event.data.action === 'next') {
            playNextSong();
        } else if (event.data.action === 'toggleLike') {
            toggleLike();
        } else if (event.data.action === 'updateSongLike') {
            updateSongListIcon(event.data.uuid, event.data.isLiked);
        }
    });

